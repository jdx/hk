#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps { ["kube-linter"] = Builtins.kube_linter }
PKL
}

teardown() {
    _common_teardown
}

@test "kube-linter builtin tests" {
    run hk test --step kube-linter
    assert_success
    assert_output --partial "ok - kube-linter :: check unsafe pod"
    assert_output --partial "ok - kube-linter :: check excludes chart metadata and templates"
}

@test "kube-linter scopes plain manifests and reports policy violations" {
    mkdir -p 'services/example/k8s' .github/workflows
    cat > 'services/example/k8s/unsafe pod.yaml' <<'YAML'
apiVersion: v1
kind: Pod
metadata:
  name: example
spec:
  containers:
    - name: app
      image: nginx:latest
YAML
    cp 'services/example/k8s/unsafe pod.yaml' .github/workflows/not-a-manifest.yml
    run hk check --check .github/workflows/not-a-manifest.yml
    assert_success

    run hk check --check 'services/example/k8s/unsafe pod.yaml'
    assert_failure
    assert_output --partial "latest-tag"
}
