#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

run_tests() {
    for test_name in \
        test_large_directory_perf \
        test_filter_eval_perf \
        test_peer_cache_lookup_perf
    do
        cargo test -q -p syncweb-core --test integration_tests \
            "$test_name" -- --nocapture
    done
    if [[ -x /usr/bin/time ]]; then
        /usr/bin/time \
            -f 'scale resources: user_cpu=%U sys_cpu=%S max_rss_kb=%M' \
            cargo test -q -p syncweb-core \
            swarm_simulator::tests::scale_fixture_covers_lan_gate_sizes \
            -- --nocapture
    else
        cargo test -q -p syncweb-core \
            swarm_simulator::tests::scale_fixture_covers_lan_gate_sizes \
            -- --nocapture
    fi
}

case "${1:-tests}" in
    tests)
        run_tests
        ;;
    bench)
        cargo bench -q -p syncweb-core --bench core_benchmarks
        ;;
    all)
        run_tests
        cargo bench -q -p syncweb-core --bench core_benchmarks
        ;;
    *)
        printf 'usage: %s [tests|bench|all]\n' "$0" >&2
        exit 2
        ;;
esac
