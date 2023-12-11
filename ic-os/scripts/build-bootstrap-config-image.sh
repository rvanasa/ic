#!/usr/bin/env bash

set -e

function usage() {
    cat >&2 <<EOF
build-bootstrap-config-image.sh [-t] out_file [parameters]

Build the configuration image injected into the guest OS
during bootstrap.

The first argument may optionally be "-t" to instruct the script to just build
the tar file that contains the config information. Otherwise, it will build the
disk image that will be injected as removable media into the bootstrap process.

The output file needs to be given next. The script will either write a
disk image or tar file as output file (see above).

Following that are the options specifying the configuration to write. Each of
option takes a value given as next argument, and any number of the following
options may be specified:

  --ipv6_address a:b::c/n
    The IPv6 address to assign. Must include netmask in bits (e.g.
    dead:beef::1/64). Overrides all other generation for testing.

  --ipv6_gateway a:b::c
    Default IPv6 gateway.

  --ipv6_name_servers servers
    ipv6 DNS servers to use. Can be multiple servers separated by space (make
    sure to quote the argument string so it appears as a single argument to the
    script, e.g. --ipv6_name_servers "2606:4700:4700::1111
    2606:4700:4700::1001").

  --ipv4_name_servers servers
    ipv4 DNS servers to use. Can be multiple servers separated by space (make
    sure to quote the argument string so it appears as a single argument to the
    script, e.g. --ipv4_name_servers "1.1.1.1 1.0.0.1").

  --ipv4_address a.b.c.d/n
    (optional) The IPv4 address to assign. Must include prefix length (e.g.
    18.208.190.35/28).

  --ipv4_gateway a:b::c
    (optional) Default IPv4 gateway (e.g. 18.208.190.33).

  --hostname name
    Name to assign to the host. Will be used in logging.

  --ic_crypto path
    Injected crypto state. Should point to a directory containing material
    generated by ic-prep. Typically, this is IC_PREP_OUT_PATH/node-X/crypto.

  --ic_registry_local_store path
    Injected initial registry state. Should point to a directory containing
    material generated by ic-prep. Typically, this is
    IC_PREP_OUT_PATH/ic_registry_local_store

  --elasticsearch_hosts hosts
    Logging hosts to use. Can be multiple hosts separated by space (make sure
    to quote the argument string so it appears as a single argument to the
    script, e.g. --elasticsearch_hosts "h1.domain.tld h2.domain.tld").

  --elasticsearch_tags tags
    Tags to be used by Filebeat. Can be multiple tags separated by space
    (make sure to quote the argument string so it appears as a single argument
    to the script, e.g. --elasticsearch_tags "testnet1 slo")

  --nns_url url
    URL of NNS nodes for sign up or registry access. Can be multiple nodes
    separated by commas.

  --nns_public_key path
    NNS public key file.

  --accounts_ssh_authorized_keys path
    Should point to a directory with files containing the authorized ssh
    keys for specific user accounts on the machine. The name of the
    key designates the name of the account (so, if there is a file
    "PATH/admin" then it is transferred to "~admin/.ssh/authorized_keys" on
    the target). The presently recognized accounts are: backup, readonly,
    admin and root (the latter one for testing purposes only!)

  --node_operator_private_key path
    Should point to a file containing a Node Provider private key PEM.

  --backup_retention_time seconds
    How long the backed up consensus artifacts should stay on the spool
    before they get purged.

  --backup_puging_interval seconds
    How often the backup purging should be executed.

  --replica_log_debug_overrides overrides
    A list of fully qualified Rust module paths. For each of the listed
    modules, at least DEBUG logs will be produced by the node software.
    Primarily intended for testing.

    The list must be provided as a serialized JSON-array. The value is
    inserted into the configuration file as is. E.g.:
      '["ic_consensus::consensus::finalizer",\
       "ic_consensus::consensus::catchup_package_maker"]'

    Be sure to properly quote the string.

  --malicious_behavior malicious_behavior
    A JSON-object that describes the malicious behavior activated on
    the node. This is only used for testing.

    The Json-object corresponds to this Rust-structure:
      ic_types::malicious_behaviour::MaliciousBehaviour

  --bitcoind_addr address
    The IP address of a running bitcoind instance. To be used in
    systems tests only.

  --socks_proxy url
    The URL of the socks proxy to use. To be used in
    systems tests only.

  --get_sev_certs
    If on an SEV-SNP enabled machine, include the ark, ask, and vcek
    certificates in the config image.  Note: this requires that this
    script is executed on the host which will be running the SEV-SNP VM.
EOF
}

# Arguments:
# - $1 the tar file to build
# - all remaining arguments: parameters to encode into the bootstrap
function build_ic_bootstrap_tar() {
    local OUT_FILE="$1"
    shift

    local IPV6_ADDRESS IPV6_GATEWAY IPV6_NAME_SERVERS IPV4_NAME_SERVERS HOSTNAME
    local IC_CRYPTO IC_REGISTRY_LOCAL_STORE
    local NNS_URL NNS_PUBLIC_KEY NODE_OPERATOR_PRIVATE_KEY
    local BACKUP_RETENTION_TIME_SECS BACKUP_PURGING_INTERVAL_SECS
    local ELASTICSEARCH_HOSTS ELASTICSEARCH_TAGS
    local ACCOUNTS_SSH_AUTHORIZED_KEYS
    local REPLICA_LOG_DEBUG_OVERRIDES
    local MALICIOUS_BEHAVIOR
    local BITCOIND_ADDR
    local GET_SEV_CERTS=false

    while true; do
        if [ $# == 0 ]; then
            break
        fi
        case "$1" in
            --ipv6_address)
                IPV6_ADDRESS="$2"
                ;;
            --ipv6_gateway)
                IPV6_GATEWAY="$2"
                ;;
            --ipv6_name_servers)
                IPV6_NAME_SERVERS="$2"
                ;;
            --ipv4_name_servers)
                IPV4_NAME_SERVERS="$2"
                ;;
            --ipv4_address)
                IPV4_ADDRESS="$2"
                ;;
            --ipv4_gateway)
                IPV4_GATEWAY="$2"
                ;;
            --hostname)
                HOSTNAME="$2"
                ;;
            --ic_crypto)
                IC_CRYPTO="$2"
                ;;
            --ic_registry_local_store)
                IC_REGISTRY_LOCAL_STORE="$2"
                ;;
            --elasticsearch_hosts)
                ELASTICSEARCH_HOSTS="$2"
                ;;
            --elasticsearch_tags)
                ELASTICSEARCH_TAGS="$2"
                ;;
            --nns_url)
                NNS_URL="$2"
                ;;
            --nns_public_key)
                NNS_PUBLIC_KEY="$2"
                ;;
            --accounts_ssh_authorized_keys)
                ACCOUNTS_SSH_AUTHORIZED_KEYS="$2"
                ;;
            --node_operator_private_key)
                NODE_OPERATOR_PRIVATE_KEY="$2"
                ;;
            --backup_retention_time)
                BACKUP_RETENTION_TIME_SECS="$2"
                ;;
            --backup_puging_interval)
                BACKUP_PURGING_INTERVAL_SECS="$2"
                ;;
            --replica_log_debug_overrides)
                REPLICA_LOG_DEBUG_OVERRIDES="$2"
                ;;
            --malicious_behavior)
                MALICIOUS_BEHAVIOR="$2"
                ;;
            --bitcoind_addr)
                BITCOIND_ADDR="$2"
                ;;
            --socks_proxy)
                SOCKS_PROXY="$2"
                ;;
            --get_sev_certs)
                GET_SEV_CERTS=true
                shift 1
                continue
                ;;
            *)
                echo "Unrecognized option: $1"
                usage
                exit 1
                break
                ;;
        esac
        shift 2
    done

    [[ "$HOSTNAME" == "" ]] || [[ "$HOSTNAME" =~ [a-zA-Z]*([a-zA-Z0-9])*(-+([a-zA-Z0-9])) ]] || {
        echo "Invalid hostname: '$HOSTNAME'" >&2
        exit 1
    }

    local BOOTSTRAP_TMPDIR=$(mktemp -d)

    cat >"${BOOTSTRAP_TMPDIR}/network.conf" <<EOF
${IPV6_ADDRESS:+ipv6_address=$IPV6_ADDRESS}
${IPV6_GATEWAY:+ipv6_gateway=$IPV6_GATEWAY}
name_servers=$IPV6_NAME_SERVERS
ipv4_name_servers=$IPV4_NAME_SERVERS
hostname=$HOSTNAME
${IPV4_ADDRESS:+ipv4_address=$IPV4_ADDRESS}
${IPV4_GATEWAY:+ipv4_gateway=$IPV4_GATEWAY}
EOF
    if [ "${ELASTICSEARCH_HOSTS}" != "" ]; then
        echo "elasticsearch_hosts=$ELASTICSEARCH_HOSTS" >"${BOOTSTRAP_TMPDIR}/filebeat.conf"
    fi
    if [ "${ELASTICSEARCH_TAGS}" != "" ]; then
        echo "elasticsearch_tags=$ELASTICSEARCH_TAGS" >>"${BOOTSTRAP_TMPDIR}/filebeat.conf"
    fi
    if [ "${NNS_PUBLIC_KEY}" != "" ]; then
        cp "${NNS_PUBLIC_KEY}" "${BOOTSTRAP_TMPDIR}/nns_public_key.pem"
    fi
    if [ "${NNS_URL}" != "" ]; then
        echo "nns_url=${NNS_URL}" >"${BOOTSTRAP_TMPDIR}/nns.conf"
    fi
    if [ "${BACKUP_RETENTION_TIME_SECS}" != "" ] || [ "${BACKUP_PURGING_INTERVAL_SECS}" != "" ]; then
        echo "backup_retention_time_secs=${BACKUP_RETENTION_TIME_SECS}" >"${BOOTSTRAP_TMPDIR}/backup.conf"
        echo "backup_puging_interval_secs=${BACKUP_PURGING_INTERVAL_SECS}" >>"${BOOTSTRAP_TMPDIR}/backup.conf"
    fi
    if [ "${REPLICA_LOG_DEBUG_OVERRIDES}" != "" ]; then
        echo "replica_log_debug_overrides=${REPLICA_LOG_DEBUG_OVERRIDES}" >"${BOOTSTRAP_TMPDIR}/log.conf"
    fi
    if [ "${MALICIOUS_BEHAVIOR}" != "" ]; then
        echo "malicious_behavior=${MALICIOUS_BEHAVIOR}" >"${BOOTSTRAP_TMPDIR}/malicious_behavior.conf"
    fi
    if [ "${BITCOIND_ADDR}" != "" ]; then
        echo "bitcoind_addr=${BITCOIND_ADDR}" >"${BOOTSTRAP_TMPDIR}/bitcoind_addr.conf"
    fi
    if [ "${SOCKS_PROXY}" != "" ]; then
        echo "socks_proxy=${SOCKS_PROXY}" >"${BOOTSTRAP_TMPDIR}/socks_proxy.conf"
    fi
    if [ "${IC_CRYPTO}" != "" ]; then
        cp -r "${IC_CRYPTO}" "${BOOTSTRAP_TMPDIR}/ic_crypto"
    fi
    if [ "${IC_REGISTRY_LOCAL_STORE}" != "" ]; then
        cp -r "${IC_REGISTRY_LOCAL_STORE}" "${BOOTSTRAP_TMPDIR}/ic_registry_local_store"
    fi
    if [ "${ACCOUNTS_SSH_AUTHORIZED_KEYS}" != "" ]; then
        cp -r "${ACCOUNTS_SSH_AUTHORIZED_KEYS}" "${BOOTSTRAP_TMPDIR}/accounts_ssh_authorized_keys"
    fi
    if [ "${NODE_OPERATOR_PRIVATE_KEY}" != "" ]; then
        cp "${NODE_OPERATOR_PRIVATE_KEY}" "${BOOTSTRAP_TMPDIR}/node_operator_private_key.pem"
    fi
    if [[ "${GET_SEV_CERTS}" == true && ! -e "/dev/sev" ]]; then
        echo "--get_sev_certs is true but /dev/sev is not available, unable to get SEV certs"
    fi
    if [[ "${GET_SEV_CERTS}" == true && -e "/dev/sev" ]]; then
        /opt/ic/bin/get-sev-certs.sh
    fi

    tar cf "${OUT_FILE}" -C "${BOOTSTRAP_TMPDIR}" .

    rm -rf "${BOOTSTRAP_TMPDIR}"
}

# Arguments:
# - $1 the disk image to be built
# - all remaining arguments: parameters to encode into the bootstrap

function build_ic_bootstrap_diskimage() {
    local OUT_FILE="$1"
    shift

    local TMPDIR=$(mktemp -d)
    local TAR="${TMPDIR}/ic-bootstrap.tar"
    build_ic_bootstrap_tar "${TAR}" "$@"

    size=$(du --bytes "${TAR}" | awk '{print $1}')
    size=$((2 * size + 1048576))
    echo "image size: $size"
    truncate -s $size "${OUT_FILE}"
    mkfs.vfat -n CONFIG "${OUT_FILE}"
    mcopy -i "${OUT_FILE}" -o "${TAR}" ::

    rm -rf "${TMPDIR}"
}

BUILD_TAR_ONLY=0
if [ "$1" == "-t" -o "$1" == "--tar" ]; then
    BUILD_TAR_ONLY=1
    shift
fi

if [ "$#" -lt 2 ]; then
    usage
    exit 1
fi

if [ "${BUILD_TAR_ONLY}" == 0 ]; then
    build_ic_bootstrap_diskimage "$@"
else
    build_ic_bootstrap_tar "$@"
fi
