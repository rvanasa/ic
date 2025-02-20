mod ecdsa_sign_share {
    use crate::key_id::KeyId;
    use crate::secret_key_store::mock_secret_key_store::MockSecretKeyStore;
    use crate::types::CspSecretKey;
    use crate::vault::api::ThresholdEcdsaSignerCspVault;
    use crate::LocalCspVault;
    use assert_matches::assert_matches;
    use ic_crypto_internal_threshold_sig_ecdsa::{
        CombinedCommitment, CommitmentOpeningBytes, EccCurveType, EccPoint, EccScalarBytes,
        IDkgTranscriptInternal, PolynomialCommitment, SimpleCommitment,
        ThresholdEcdsaSigShareInternal,
    };
    use ic_types::crypto::canister_threshold_sig::error::ThresholdEcdsaSignShareError;
    use ic_types::crypto::canister_threshold_sig::ExtendedDerivationPath;
    use ic_types::crypto::AlgorithmId;
    use ic_types::Randomness;
    use proptest::collection::vec;
    use proptest::prelude::any;
    use proptest::proptest;
    use proptest::strategy::Strategy;
    use serde_bytes::ByteBuf;
    use std::collections::HashSet;

    #[test]
    fn should_error_when_lambda_masked_not_found() {
        let parameters = EcdsaSignShareParameters::default();
        let mut canister_sks = MockSecretKeyStore::new();
        parameters.without_lambda_masked_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = parameters.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::SecretSharesNotFound { commitment_string })
            if commitment_string == format!("{:?}", parameters.lambda_masked.combined_commitment.commitment())
        )
    }

    #[test]
    fn should_error_when_lambda_masked_has_wrong_type() {
        let parameters = EcdsaSignShareParameters::default();
        let lambda_masked_key_id =
            KeyId::from(parameters.lambda_masked.combined_commitment.commitment());

        proptest!(|(lambda_masked_sk in arb_csp_secret_key_with_wrong_type())| {
            let mut canister_sks = MockSecretKeyStore::new();
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == lambda_masked_key_id)
                .return_const(Some(lambda_masked_sk));
            let vault = LocalCspVault::builder_for_test()
                .with_mock_stores()
                .with_canister_secret_key_store(canister_sks)
                .build();

            let result = parameters.ecdsa_sign_share(&vault);

            assert_matches!(
                result,
                Err(ThresholdEcdsaSignShareError::SecretSharesNotFound { commitment_string })
                if commitment_string == format!("{:?}", parameters.lambda_masked.combined_commitment.commitment())
            )
        });
    }

    #[test]
    fn should_error_when_kappa_times_lambda_not_found() {
        let parameters = EcdsaSignShareParameters::default();
        let mut canister_sks = MockSecretKeyStore::new();
        parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
        parameters.without_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = parameters.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::SecretSharesNotFound { commitment_string })
            if commitment_string == format!("{:?}", parameters.kappa_times_lambda.combined_commitment.commitment())
        )
    }

    #[test]
    fn should_error_when_kappa_times_lambda_has_wrong_type() {
        let parameters = EcdsaSignShareParameters::default();
        let kappa_times_lambda_key_id = KeyId::from(
            parameters
                .kappa_times_lambda
                .combined_commitment
                .commitment(),
        );

        proptest!(|(kappa_times_lambda_sk in arb_csp_secret_key_with_wrong_type())| {
            let mut canister_sks = MockSecretKeyStore::new();
            parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == kappa_times_lambda_key_id)
                .return_const(Some(kappa_times_lambda_sk));
            let vault = LocalCspVault::builder_for_test()
                .with_mock_stores()
                .with_canister_secret_key_store(canister_sks)
                .build();

            let result = parameters.ecdsa_sign_share(&vault);

            assert_matches!(
                result,
                Err(ThresholdEcdsaSignShareError::SecretSharesNotFound { commitment_string })
                if commitment_string == format!("{:?}", parameters.kappa_times_lambda.combined_commitment.commitment())
            )
        });
    }

    #[test]
    fn should_error_when_key_times_lambda_not_found() {
        let parameters = EcdsaSignShareParameters::default();
        let mut canister_sks = MockSecretKeyStore::new();
        parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
        parameters.with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        parameters.without_key_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = parameters.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::SecretSharesNotFound { commitment_string })
            if commitment_string == format!("{:?}", parameters.key_times_lambda.combined_commitment.commitment())
        )
    }

    #[test]
    fn should_error_when_key_times_lambda_has_wrong_type() {
        let parameters = EcdsaSignShareParameters::default();
        let key_times_lambda_key_id =
            KeyId::from(parameters.key_times_lambda.combined_commitment.commitment());

        proptest!(|(key_times_lambda_sk in arb_csp_secret_key_with_wrong_type())| {
            let mut canister_sks = MockSecretKeyStore::new();
            parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
            parameters.with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == key_times_lambda_key_id)
                .return_const(Some(key_times_lambda_sk));
            let vault = LocalCspVault::builder_for_test()
                .with_mock_stores()
                .with_canister_secret_key_store(canister_sks)
                .build();

            let result = parameters.ecdsa_sign_share(&vault);

            assert_matches!(
                result,
                Err(ThresholdEcdsaSignShareError::SecretSharesNotFound { commitment_string })
                if commitment_string == format!("{:?}", parameters.key_times_lambda.combined_commitment.commitment())
            )
        });
    }

    #[test]
    fn should_error_when_algorithm_id_wrong() {
        use strum::IntoEnumIterator;

        AlgorithmId::iter()
            .filter(|algorithm_id| !algorithm_id.is_threshold_ecdsa())
            .for_each(|wrong_algorithm_id| {
                let parameters = EcdsaSignShareParameters::default();
                let mut canister_sks = MockSecretKeyStore::new();
                parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
                parameters.with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
                parameters.with_key_times_lambda_idkg_commitment(&mut canister_sks);
                let vault = LocalCspVault::builder_for_test()
                    .with_mock_stores()
                    .with_canister_secret_key_store(canister_sks)
                    .build();

                let parameters_with_wrong_algorithm_id = EcdsaSignShareParameters {
                    algorithm_id: wrong_algorithm_id,
                    ..parameters
                };
                let result = parameters_with_wrong_algorithm_id.ecdsa_sign_share(&vault);

                assert_matches!(
                    result,
                    Err(ThresholdEcdsaSignShareError::InternalError { internal_error })
                    if internal_error.contains("UnsupportedAlgorithm")
                )
            });
    }

    #[test]
    fn should_error_when_hashed_message_is_not_32_bytes() {
        proptest!(|(hashed_message in vec(any::<u8>(), 0..100)
            .prop_filter("hashed_message must not be 32 bytes", |s| s.len() != 32))| {
            let parameters = EcdsaSignShareParameters::default();
            let mut canister_sks = MockSecretKeyStore::new();
            parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
            parameters.with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
            parameters.with_key_times_lambda_idkg_commitment(&mut canister_sks);
            let vault = LocalCspVault::builder_for_test()
                .with_mock_stores()
                .with_canister_secret_key_store(canister_sks)
                .build();

            let parameters_with_invalid_hashed_message_length = EcdsaSignShareParameters {
                hashed_message,
                ..parameters
            };
            let result = parameters_with_invalid_hashed_message_length.ecdsa_sign_share(&vault);

            //TODO CRP-1340 add dedicated error type for hash length mismatch
            assert_matches!(
                result,
                Err(ThresholdEcdsaSignShareError::InternalError { internal_error })
                if internal_error.contains("UnsupportedAlgorithm")
            )
        });
    }

    #[test]
    fn should_error_when_kappa_unmasked_commitment_type_wrong() {
        let parameters = EcdsaSignShareParameters::default();
        let mut canister_sks = MockSecretKeyStore::new();
        parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
        parameters.with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        parameters.with_key_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let params_with_wrong_commitment_type = {
            let kappa_unmasked_by_summation = IDkgTranscriptInternal {
                combined_commitment: CombinedCommitment::BySummation(
                    parameters
                        .kappa_unmasked
                        .combined_commitment
                        .commitment()
                        .clone(),
                ),
            };
            EcdsaSignShareParameters {
                kappa_unmasked: kappa_unmasked_by_summation,
                ..parameters
            }
        };

        let result = params_with_wrong_commitment_type.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::InternalError { internal_error })
            if internal_error.contains("UnexpectedCommitmentType")
        )
    }

    #[test]
    fn should_error_when_lambda_masked_commitment_is_simple() {
        let params_with_wrong_lambda_masked_commitment = {
            let parameters = EcdsaSignShareParameters::default();
            let simple_opening = match parameters.lambda_masked_commitment {
                CspSecretKey::IDkgCommitmentOpening(ref opening_bytes) => {
                    to_simple_commitment_opening(opening_bytes)
                }
                _ => panic!("Wrong secret key type"),
            };
            let lambda_masked_commitment = CspSecretKey::IDkgCommitmentOpening(simple_opening);
            EcdsaSignShareParameters {
                lambda_masked_commitment,
                ..parameters
            }
        };
        let mut canister_sks = MockSecretKeyStore::new();
        params_with_wrong_lambda_masked_commitment
            .with_lambda_masked_idkg_commitment(&mut canister_sks);
        params_with_wrong_lambda_masked_commitment
            .with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        params_with_wrong_lambda_masked_commitment
            .with_key_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = params_with_wrong_lambda_masked_commitment.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::InternalError { internal_error })
            if internal_error.contains("UnexpectedCommitmentType")
        )
    }

    #[test]
    fn should_error_when_kappa_times_lambda_commitment_is_simple() {
        let params_with_wrong_kappa_times_lambda_commitment = {
            let parameters = EcdsaSignShareParameters::default();
            let simple_opening = match parameters.kappa_times_lambda_commitment {
                CspSecretKey::IDkgCommitmentOpening(ref opening_bytes) => {
                    to_simple_commitment_opening(opening_bytes)
                }
                _ => panic!("Wrong secret key type"),
            };
            let kappa_times_lambda_commitment = CspSecretKey::IDkgCommitmentOpening(simple_opening);
            EcdsaSignShareParameters {
                kappa_times_lambda_commitment,
                ..parameters
            }
        };
        let mut canister_sks = MockSecretKeyStore::new();
        params_with_wrong_kappa_times_lambda_commitment
            .with_lambda_masked_idkg_commitment(&mut canister_sks);
        params_with_wrong_kappa_times_lambda_commitment
            .with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        params_with_wrong_kappa_times_lambda_commitment
            .with_key_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = params_with_wrong_kappa_times_lambda_commitment.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::InternalError { internal_error })
            if internal_error.contains("UnexpectedCommitmentType")
        )
    }

    #[test]
    fn should_error_when_key_times_lambda_commitment_is_simple() {
        let params_with_wrong_key_times_lambda_commitment = {
            let parameters = EcdsaSignShareParameters::default();
            let simple_opening = match parameters.key_times_lambda_commitment {
                CspSecretKey::IDkgCommitmentOpening(ref opening_bytes) => {
                    to_simple_commitment_opening(opening_bytes)
                }
                _ => panic!("Wrong secret key type"),
            };
            let key_times_lambda_commitment = CspSecretKey::IDkgCommitmentOpening(simple_opening);
            EcdsaSignShareParameters {
                key_times_lambda_commitment,
                ..parameters
            }
        };
        let mut canister_sks = MockSecretKeyStore::new();
        params_with_wrong_key_times_lambda_commitment
            .with_lambda_masked_idkg_commitment(&mut canister_sks);
        params_with_wrong_key_times_lambda_commitment
            .with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        params_with_wrong_key_times_lambda_commitment
            .with_key_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = params_with_wrong_key_times_lambda_commitment.ecdsa_sign_share(&vault);

        assert_matches!(
            result,
            Err(ThresholdEcdsaSignShareError::InternalError { internal_error })
            if internal_error.contains("UnexpectedCommitmentType")
        )
    }
    #[test]
    fn should_sign_share() {
        let parameters = EcdsaSignShareParameters::default();
        let mut canister_sks = MockSecretKeyStore::new();
        parameters.with_lambda_masked_idkg_commitment(&mut canister_sks);
        parameters.with_kappa_times_lambda_idkg_commitment(&mut canister_sks);
        parameters.with_key_times_lambda_idkg_commitment(&mut canister_sks);
        let vault = LocalCspVault::builder_for_test()
            .with_mock_stores()
            .with_canister_secret_key_store(canister_sks)
            .build();

        let result = parameters.ecdsa_sign_share(&vault);

        assert_matches!(result, Ok(_))
    }

    fn some_derivation_path() -> ExtendedDerivationPath {
        ExtendedDerivationPath {
            caller: Default::default(),
            derivation_path: vec![],
        }
    }

    struct EcdsaSignShareParameters {
        derivation_path: ExtendedDerivationPath,
        hashed_message: Vec<u8>,
        nonce: Randomness,
        key: IDkgTranscriptInternal,
        kappa_unmasked: IDkgTranscriptInternal,
        lambda_masked: IDkgTranscriptInternal,
        kappa_times_lambda: IDkgTranscriptInternal,
        key_times_lambda: IDkgTranscriptInternal,
        lambda_masked_commitment: CspSecretKey,
        kappa_times_lambda_commitment: CspSecretKey,
        key_times_lambda_commitment: CspSecretKey,
        algorithm_id: AlgorithmId,
    }

    impl Default for EcdsaSignShareParameters {
        fn default() -> Self {
            let [key, kappa_unmasked, lambda_masked, kappa_times_lambda, key_times_lambda] =
                distinct_transcripts();
            let kappa_unmasked = IDkgTranscriptInternal {
                combined_commitment: CombinedCommitment::ByInterpolation(
                    kappa_unmasked.combined_commitment.commitment().clone(),
                ),
            };
            let [lambda_masked_commitment, kappa_times_lambda_commitment, key_times_lambda_commitment] =
                distinct_idkg_commitment_openings();

            EcdsaSignShareParameters {
                derivation_path: some_derivation_path(),
                hashed_message: "hello world on thirty-two bytes!".as_bytes().to_vec(),
                nonce: Randomness::from([0; 32]),
                key,
                kappa_unmasked,
                lambda_masked,
                kappa_times_lambda,
                key_times_lambda,
                lambda_masked_commitment,
                kappa_times_lambda_commitment,
                key_times_lambda_commitment,
                algorithm_id: AlgorithmId::ThresholdEcdsaSecp256k1,
            }
        }
    }

    impl EcdsaSignShareParameters {
        fn ecdsa_sign_share<V: ThresholdEcdsaSignerCspVault>(
            &self,
            vault: &V,
        ) -> Result<ThresholdEcdsaSigShareInternal, ThresholdEcdsaSignShareError> {
            vault.ecdsa_sign_share(
                &self.derivation_path,
                &self.hashed_message,
                &self.nonce,
                ByteBuf::from(self.key.serialize().expect("should serialize successfully")),
                ByteBuf::from(
                    self.kappa_unmasked
                        .serialize()
                        .expect("should serialize successfully"),
                ),
                ByteBuf::from(
                    self.lambda_masked
                        .serialize()
                        .expect("should serialize successfully"),
                ),
                ByteBuf::from(
                    self.kappa_times_lambda
                        .serialize()
                        .expect("should serialize successfully"),
                ),
                ByteBuf::from(
                    self.key_times_lambda
                        .serialize()
                        .expect("should serialize successfully"),
                ),
                self.algorithm_id,
            )
        }

        fn without_lambda_masked_idkg_commitment(&self, canister_sks: &mut MockSecretKeyStore) {
            let lambda_masked_key_id =
                KeyId::from(self.lambda_masked.combined_commitment.commitment());
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == lambda_masked_key_id)
                .return_const(None);
        }

        fn without_kappa_times_lambda_idkg_commitment(
            &self,
            canister_sks: &mut MockSecretKeyStore,
        ) {
            let kappa_times_lambda_key_id =
                KeyId::from(self.kappa_times_lambda.combined_commitment.commitment());
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == kappa_times_lambda_key_id)
                .return_const(None);
        }

        fn without_key_times_lambda_idkg_commitment(&self, canister_sks: &mut MockSecretKeyStore) {
            let key_times_lambda_key_id =
                KeyId::from(self.key_times_lambda.combined_commitment.commitment());
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == key_times_lambda_key_id)
                .return_const(None);
        }

        fn with_lambda_masked_idkg_commitment(&self, canister_sks: &mut MockSecretKeyStore) {
            let lambda_masked_key_id =
                KeyId::from(self.lambda_masked.combined_commitment.commitment());
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == lambda_masked_key_id)
                .return_const(Some(self.lambda_masked_commitment.clone()));
        }

        fn with_kappa_times_lambda_idkg_commitment(&self, canister_sks: &mut MockSecretKeyStore) {
            let kappa_times_lambda_key_id =
                KeyId::from(self.kappa_times_lambda.combined_commitment.commitment());
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == kappa_times_lambda_key_id)
                .return_const(Some(self.kappa_times_lambda_commitment.clone()));
        }

        fn with_key_times_lambda_idkg_commitment(&self, canister_sks: &mut MockSecretKeyStore) {
            let key_times_lambda_key_id =
                KeyId::from(self.key_times_lambda.combined_commitment.commitment());
            canister_sks
                .expect_get()
                .times(1)
                .withf(move |key_id| *key_id == key_times_lambda_key_id)
                .return_const(Some(self.key_times_lambda_commitment.clone()));
        }
    }

    fn distinct_transcripts<const N: usize>() -> [IDkgTranscriptInternal; N] {
        // IDkgTranscriptInternal does not implement Hash so
        // to ensure generated transcripts are distinct, we insert their serialized form in a HashSet
        let mut serialized_transcripts = HashSet::with_capacity(N);
        let transcripts: Vec<_> = distinct_ecc_points(N)
            .into_iter()
            .map(|point| {
                let transcript = IDkgTranscriptInternal {
                    combined_commitment: CombinedCommitment::BySummation(
                        PolynomialCommitment::from(SimpleCommitment {
                            points: vec![point],
                        }),
                    ),
                };
                assert!(serialized_transcripts
                    .insert(transcript.serialize().expect("can serialize transcript")));
                transcript
            })
            .collect();
        assert_eq!(
            serialized_transcripts.len(),
            N,
            "Duplicate transcripts generated"
        );

        transcripts
            .try_into()
            .map_err(|_err| "failed to convert to fixed size array")
            .expect("failed to convert to fixed size array")
    }

    fn distinct_ecc_points(num_points: usize) -> Vec<EccPoint> {
        // EccPoint does not implement Hash so
        // to ensure generated EccPoints are distinct, we insert their serialized form in a HashSet
        let mut serialized_points = HashSet::new();
        let mut points = Vec::with_capacity(num_points);
        let mut current_point = some_ecc_point();
        for _ in 0..num_points {
            current_point = current_point
                .add_points(&some_ecc_point())
                .expect("add_points failed");
            assert!(
                serialized_points.insert(current_point.serialize()),
                "Duplicate point {:?} generated",
                current_point
            );
            points.push(current_point.clone());
        }
        assert_eq!(
            serialized_points.len(),
            num_points,
            "Duplicate points generated"
        );
        assert_eq!(points.len(), num_points);
        points
    }

    fn some_ecc_point() -> EccPoint {
        EccPoint::generator_g(EccCurveType::K256)
    }

    fn distinct_idkg_commitment_openings<const N: usize>() -> [CspSecretKey; N] {
        assert!(u8::try_from(N).is_ok(), "N must be less than 256");
        let mut openings = Vec::with_capacity(N);
        for i in 0..N {
            let some_scalar = EccScalarBytes::K256(Box::new(
                [u8::try_from(i).expect("index should fit in u8"); 32],
            ));
            openings.push(CspSecretKey::IDkgCommitmentOpening(
                CommitmentOpeningBytes::Pedersen(some_scalar.clone(), some_scalar),
            ));
        }
        openings
            .try_into()
            .map_err(|_err| "failed to convert to fixed size array")
            .expect("failed to convert to fixed size array")
    }

    fn arb_csp_secret_key_with_wrong_type() -> impl Strategy<Value = CspSecretKey> {
        any::<CspSecretKey>().prop_filter(
            "Secret key must not be of type IDkgCommitmentOpening",
            |sk| !matches!(sk, CspSecretKey::IDkgCommitmentOpening(_)),
        )
    }

    fn to_simple_commitment_opening(commitment: &CommitmentOpeningBytes) -> CommitmentOpeningBytes {
        let scalar_bytes = match commitment {
            CommitmentOpeningBytes::Simple(bytes) => bytes,
            CommitmentOpeningBytes::Pedersen(bytes, _) => bytes,
        };
        CommitmentOpeningBytes::Simple(scalar_bytes.clone())
    }
}
