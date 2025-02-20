use crate::eth_rpc::JsonRpcResult;
use crate::eth_rpc::{
    BlockSpec, BlockTag, FeeHistory, FeeHistoryParams, Quantity, SendRawTransactionResult,
};
use crate::eth_rpc_client::requests::GetTransactionCountParams;
use crate::eth_rpc_client::responses::TransactionReceipt;
use crate::eth_rpc_client::EthRpcClient;
use crate::eth_rpc_client::MultiCallError;
use crate::guard::TimerGuard;
use crate::logs::{DEBUG, INFO};
use crate::numeric::{LedgerBurnIndex, LedgerMintIndex, TransactionCount};
use crate::state::audit::{process_event, EventType};
use crate::state::transactions::{
    create_transaction, CreateTransactionError, Reimbursed, ReimbursementRequest,
};
use crate::state::{mutate_state, read_state, State, TaskType};
use crate::tx::{estimate_transaction_price, TransactionPrice};
use candid::Nat;
use futures::future::join_all;
use ic_canister_log::log;
use icrc_ledger_client_cdk::{CdkRuntime, ICRC1Client};
use icrc_ledger_types::icrc1::{account::Account, transfer::TransferArg};
use num_traits::ToPrimitive;
use std::collections::{BTreeMap, BTreeSet};
use std::iter::zip;

const WITHDRAWAL_REQUESTS_BATCH_SIZE: usize = 5;
const TRANSACTIONS_TO_SIGN_BATCH_SIZE: usize = 5;
const TRANSACTIONS_TO_SEND_BATCH_SIZE: usize = 5;

pub async fn process_reimbursement() {
    let _guard = match TimerGuard::new(TaskType::Reimbursement) {
        Ok(guard) => guard,
        Err(e) => {
            log!(DEBUG, "Failed retrieving reimbursement guard: {e:?}",);
            return;
        }
    };

    let reimbursement_requests: Vec<ReimbursementRequest> =
        read_state(|s| s.eth_transactions.get_reimbursement_requests());
    if reimbursement_requests.is_empty() {
        return;
    }

    let ledger_canister_id = read_state(|s| s.ledger_id);
    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id,
    };
    let mut error_count = 0;

    for reimbursement_request in reimbursement_requests {
        let args = TransferArg {
            from_subaccount: None,
            to: Account {
                owner: reimbursement_request.to,
                subaccount: reimbursement_request
                    .to_subaccount
                    .map(|subaccount| subaccount.0),
            },
            fee: None,
            created_at_time: None,
            memo: None,
            amount: Nat::from(reimbursement_request.reimbursed_amount),
        };
        let block_index = match client.transfer(args).await {
            Ok(Ok(block_index)) => block_index
                .0
                .to_u64()
                .expect("block index should fit into u64"),
            Ok(Err(err)) => {
                log!(INFO, "[process_reimbursement] Failed to mint ckETH {err}");
                error_count += 1;
                continue;
            }
            Err(err) => {
                log!(
                    INFO,
                    "[process_reimbursement] Failed to send a message to the ledger ({ledger_canister_id}): {err:?}"
                );
                error_count += 1;
                continue;
            }
        };
        mutate_state(|s| {
            process_event(
                s,
                EventType::ReimbursedEthWithdrawal(Reimbursed {
                    withdrawal_id: reimbursement_request.withdrawal_id,
                    reimbursed_in_block: LedgerMintIndex::new(block_index),
                    reimbursed_amount: reimbursement_request.reimbursed_amount,
                }),
            )
        });
    }
    if error_count > 0 {
        log!(
            INFO,
            "[process_reimbursement] Failed to reimburse {error_count} users, retrying later."
        );
    }
}

pub async fn process_retrieve_eth_requests() {
    let _guard = match TimerGuard::new(TaskType::RetrieveEth) {
        Ok(guard) => guard,
        Err(e) => {
            log!(
                DEBUG,
                "Failed retrieving timer guard to process ETH requests: {e:?}",
            );
            return;
        }
    };

    if read_state(|s| !s.eth_transactions.has_pending_requests()) {
        return;
    }

    let fee_history = match eth_fee_history().await {
        Ok(fee_history) => fee_history,
        Err(e) => {
            log!(
                INFO,
                "Failed retrieving fee history to process ETH requests: {e:?}",
            );
            return;
        }
    };
    let transaction_price = estimate_transaction_price(&fee_history);
    let max_transaction_fee = transaction_price.max_transaction_fee();
    log!(
        INFO,
        "[withdraw]: Estimated max transaction fee: {:?}",
        max_transaction_fee,
    );
    let latest_transaction_count = latest_transaction_count().await;
    resubmit_transactions_batch(latest_transaction_count, &transaction_price).await;
    create_transactions_batch(transaction_price);
    sign_transactions_batch().await;
    send_transactions_batch(latest_transaction_count).await;
    finalize_transactions_batch().await;

    if read_state(|s| s.eth_transactions.has_pending_requests()) {
        ic_cdk_timers::set_timer(
            crate::PROCESS_ETH_RETRIEVE_TRANSACTIONS_RETRY_INTERVAL,
            || ic_cdk::spawn(process_retrieve_eth_requests()),
        );
    }
}

async fn latest_transaction_count() -> Option<TransactionCount> {
    match read_state(EthRpcClient::from_state)
        .eth_get_transaction_count(GetTransactionCountParams {
            address: crate::state::minter_address().await,
            block: BlockSpec::Tag(BlockTag::Latest),
        })
        .await
        .reduce_with_min_by_key(|transaction_count| *transaction_count)
    {
        Ok(transaction_count) => Some(transaction_count),
        Err(e) => {
            log!(INFO, "Failed to get the latest transaction count: {e:?}");
            None
        }
    }
}
async fn resubmit_transactions_batch(
    latest_transaction_count: Option<TransactionCount>,
    transaction_price: &TransactionPrice,
) {
    if read_state(|s| s.eth_transactions.is_sent_tx_empty()) {
        return;
    }
    let latest_transaction_count = match latest_transaction_count {
        Some(latest_transaction_count) => latest_transaction_count,
        None => {
            return;
        }
    };
    let transactions_to_resubmit = read_state(|s| {
        s.eth_transactions
            .create_resubmit_transactions(latest_transaction_count, transaction_price.clone())
    });
    for result in transactions_to_resubmit {
        match result {
            Ok((withdrawal_id, transaction)) => {
                log!(
                    INFO,
                    "[resubmit_transactions_batch]: transactions to resubmit {transaction:?}"
                );
                mutate_state(|s| {
                    process_event(
                        s,
                        EventType::ReplacedTransaction {
                            withdrawal_id,
                            transaction,
                        },
                    )
                });
            }
            Err(e) => {
                log!(INFO, "Failed to resubmit transaction: {e:?}");
            }
        }
    }
}

fn create_transactions_batch(transaction_price: TransactionPrice) {
    for request in read_state(|s| {
        s.eth_transactions
            .withdrawal_requests_batch(WITHDRAWAL_REQUESTS_BATCH_SIZE)
    }) {
        log!(DEBUG, "[create_transactions_batch]: processing {request:?}",);
        let ethereum_network = read_state(State::ethereum_network);
        let nonce = read_state(|s| s.eth_transactions.next_transaction_nonce());
        match create_transaction(&request, nonce, transaction_price.clone(), ethereum_network) {
            Ok(transaction) => {
                log!(
                    DEBUG,
                    "[create_transactions_batch]: created transaction {transaction:?}",
                );

                mutate_state(|s| {
                    process_event(
                        s,
                        EventType::CreatedTransaction {
                            withdrawal_id: request.ledger_burn_index,
                            transaction,
                        },
                    );
                });
            }
            Err(CreateTransactionError::InsufficientAmount {
                ledger_burn_index,
                withdrawal_amount,
                max_transaction_fee,
            }) => {
                log!(
                    INFO,
                    "[create_transactions_batch]: Withdrawal request with burn index {ledger_burn_index} has insufficient
                amount {withdrawal_amount:?} to cover transaction fees: {max_transaction_fee:?}.
                Request moved back to end of queue."
                );
                mutate_state(|s| s.eth_transactions.reschedule_withdrawal_request(request));
            }
        };
    }
}

async fn sign_transactions_batch() {
    let transactions_batch: Vec<_> = read_state(|s| {
        s.eth_transactions
            .transactions_to_sign_batch(TRANSACTIONS_TO_SIGN_BATCH_SIZE)
    });
    log!(DEBUG, "Signing transactions {transactions_batch:?}");
    let results = join_all(
        transactions_batch
            .into_iter()
            .map(|(withdrawal_id, tx)| async move { (withdrawal_id, tx.sign().await) }),
    )
    .await;
    let mut errors = Vec::new();
    for (withdrawal_id, result) in results {
        match result {
            Ok(transaction) => mutate_state(|s| {
                process_event(
                    s,
                    EventType::SignedTransaction {
                        withdrawal_id,
                        transaction,
                    },
                )
            }),
            Err(e) => errors.push(e),
        }
    }
    if !errors.is_empty() {
        // At this point there might be a gap in transaction nonces between signed transactions, e.g.,
        // transactions 1,2,4,5 were signed, but 3 was not due to some unexpected error.
        // This means that transactions 4 and 5 are currently stuck until transaction 3 is signed.
        // However, we still proceed with transactions 4 and 5 since that way they might be mined faster
        // once transaction 3 is sent on the next iteration. Otherwise, we would need to re-sign transactions 4 and 5
        // and send them (together with transaction 3) on the next iteration.
        log!(INFO, "Errors encountered during signing: {errors:?}");
    }
}
async fn send_transactions_batch(latest_transaction_count: Option<TransactionCount>) {
    let latest_transaction_count = match latest_transaction_count {
        Some(latest_transaction_count) => latest_transaction_count,
        None => {
            return;
        }
    };
    let transactions_to_send: Vec<_> = read_state(|s| {
        s.eth_transactions
            .transactions_to_send_batch(latest_transaction_count, TRANSACTIONS_TO_SEND_BATCH_SIZE)
    });

    let rpc_client = read_state(EthRpcClient::from_state);
    let results = join_all(
        transactions_to_send
            .iter()
            .map(|tx| rpc_client.eth_send_raw_transaction(tx.raw_transaction_hex())),
    )
    .await;

    for (signed_tx, result) in zip(transactions_to_send, results) {
        log!(DEBUG, "Sent transaction {signed_tx:?}: {result:?}");
        match result {
            Ok(JsonRpcResult::Result(tx_result)) if tx_result == SendRawTransactionResult::Ok || tx_result == SendRawTransactionResult::NonceTooLow => {
                // In case of resubmission we may hit the case of SendRawTransactionResult::NonceTooLow
                // if the stuck transaction was mined in the meantime.
                // It will be cleaned-up once the transaction is finalized.
            }
            Ok(JsonRpcResult::Result(tx_result)) => log!(INFO,
                "Failed to send transaction {signed_tx:?}: {tx_result:?}. Will retry later.",
            ),
            Ok(JsonRpcResult::Error { code, message }) => log!(INFO,
                "Failed to send transaction {signed_tx:?}: {message} (error code = {code}). Will retry later.",
            ),
            Err(e) => {
                log!(INFO, "Failed to send transaction {signed_tx:?}: {e:?}. Will retry later.")
            }
        };
    }
}

async fn finalize_transactions_batch() {
    if read_state(|s| s.eth_transactions.is_sent_tx_empty()) {
        return;
    }

    match finalized_transaction_count().await {
        Ok(finalized_tx_count) => {
            let txs_to_finalize = read_state(|s| {
                s.eth_transactions
                    .sent_transactions_to_finalize(&finalized_tx_count)
            });
            let expected_finalized_withdrawal_ids: BTreeSet<_> =
                txs_to_finalize.values().cloned().collect();
            let rpc_client = read_state(EthRpcClient::from_state);
            let results = join_all(
                txs_to_finalize
                    .keys()
                    .map(|hash| rpc_client.eth_get_transaction_receipt(*hash)),
            )
            .await;
            let mut receipts: BTreeMap<LedgerBurnIndex, TransactionReceipt> = BTreeMap::new();
            for ((hash, withdrawal_id), result) in zip(txs_to_finalize, results) {
                match result {
                    Ok(Some(receipt)) => {
                        log!(DEBUG, "Received transaction receipt {receipt:?} for transaction {hash} and withdrawal ID {withdrawal_id}");
                        match receipts.get(&withdrawal_id) {
                            // by construction we never query twice the same transaction hash, which is a field in TransactionReceipt.
                            Some(existing_receipt) => {
                                log!(INFO, "ERROR: received different receipts for transaction {hash} with withdrawal ID {withdrawal_id}: {existing_receipt:?} and {receipt:?}. Will retry later");
                                return;
                            }
                            None => {
                                receipts.insert(withdrawal_id, receipt);
                            }
                        }
                    }
                    Ok(None) => {
                        log!(
                            DEBUG,
                            "Transaction {hash} for withdrawal ID {withdrawal_id} was not mined, it's probably a resubmitted transaction",
                        )
                    }
                    Err(e) => {
                        log!(
                            INFO,
                            "Failed to get transaction receipt for {hash} and withdrawal ID {withdrawal_id}: {e:?}. Will retry later",
                        );
                        return;
                    }
                }
            }
            let actual_finalized_withdrawal_ids: BTreeSet<_> = receipts.keys().cloned().collect();
            assert_eq!(
                expected_finalized_withdrawal_ids, actual_finalized_withdrawal_ids,
                "ERROR: unexpected transaction receipts for some withdrawal IDs"
            );
            for (withdrawal_id, transaction_receipt) in receipts {
                mutate_state(|s| {
                    process_event(
                        s,
                        EventType::FinalizedTransaction {
                            withdrawal_id,
                            transaction_receipt,
                        },
                    );
                });
            }
        }

        Err(e) => {
            log!(INFO, "Failed to get finalized transaction count: {e:?}");
        }
    }
}

async fn finalized_transaction_count() -> Result<TransactionCount, MultiCallError<TransactionCount>>
{
    read_state(EthRpcClient::from_state)
        .eth_get_transaction_count(GetTransactionCountParams {
            address: crate::state::minter_address().await,
            block: BlockSpec::Tag(BlockTag::Finalized),
        })
        .await
        .reduce_with_equality()
}

pub async fn eth_fee_history() -> Result<FeeHistory, MultiCallError<FeeHistory>> {
    read_state(EthRpcClient::from_state)
        .eth_fee_history(FeeHistoryParams {
            block_count: Quantity::from(5_u8),
            highest_block: BlockSpec::Tag(BlockTag::Latest),
            reward_percentiles: vec![20],
        })
        .await
}
