#!/usr/bin/env bash
# Run a vendored pgauthz matrix YAML fixture against a deployed authz-worker.
#
# Usage:  scripts/validate-fixture.sh <yaml-path> [worker-url]
# Default worker-url: https://authz-worker.gedw99.workers.dev
#
# Exit code 0 = all assertions passed, 1 = any assertion failed.

set -euo pipefail

YAML_PATH="${1:-}"
URL="${2:-https://authz-worker.gedw99.workers.dev}"

if [[ -z "$YAML_PATH" ]] || [[ ! -f "$YAML_PATH" ]]; then
  echo "usage: $0 <yaml-path> [worker-url]" >&2
  exit 2
fi

seeded=()    # track every tuple we seed so we can clean up

cleanup() {
  for entry in "${seeded[@]:-}"; do
    IFS='|' read -r ot oid rel st sid <<< "$entry"
    curl -sf -X DELETE \
      "${URL}/debug/tuple?object_type=${ot}&object_id=${oid}&relation=${rel}&subject_type=${st}&subject_id=${sid}" \
      >/dev/null 2>&1 || true
  done
}
trap cleanup EXIT

seed_tuple() {
  local obj=$1 rel=$2 sub=$3
  local ot=${obj%%:*} oid=${obj#*:}
  local st=${sub%%:*} sid=${sub#*:}
  curl -sf -X POST "${URL}/debug/tuple" \
    -H 'content-type: application/json' \
    -d "$(jq -n --arg ot "$ot" --arg oid "$oid" --arg rel "$rel" --arg st "$st" --arg sid "$sid" \
      '{object_type:$ot, object_id:$oid, relation:$rel, subject_type:$st, subject_id:$sid}')" \
    >/dev/null
  seeded+=("${ot}|${oid}|${rel}|${st}|${sid}")
}

run_check() {
  local model=$1 obj=$2 rel=$3 sub=$4
  local ot=${obj%%:*} oid=${obj#*:}
  local st=${sub%%:*} sid=${sub#*:}
  curl -sf -X POST "${URL}/check" \
    -H 'content-type: application/json' \
    -d "$(jq -n --arg model "$model" --arg ot "$ot" --arg oid "$oid" \
                --arg rel "$rel" --arg st "$st" --arg sid "$sid" \
      '{model:$model, object_type:$ot, object_id:$oid, relation:$rel, subject_type:$st, subject_id:$sid}')" \
    | jq -r '.result'
}

NAME=$(yq -r '.name' "$YAML_PATH")
MODEL=$(yq -r '.model' "$YAML_PATH")
TEST_COUNT=$(yq -r '.tests | length' "$YAML_PATH")

echo "== $NAME ($YAML_PATH against $URL) =="
echo "tests: $TEST_COUNT"
echo

pass=0
fail=0

for ((t = 0; t < TEST_COUNT; t++)); do
  test_name=$(yq -r ".tests[$t].name" "$YAML_PATH")
  echo "[$test_name]"

  # Seed setup tuples (if any)
  setup_count=$(yq -r ".tests[$t].setup.tuples // [] | length" "$YAML_PATH")
  for ((s = 0; s < setup_count; s++)); do
    obj=$(yq -r ".tests[$t].setup.tuples[$s].object" "$YAML_PATH")
    rel=$(yq -r ".tests[$t].setup.tuples[$s].relation" "$YAML_PATH")
    sub=$(yq -r ".tests[$t].setup.tuples[$s].subject" "$YAML_PATH")
    seed_tuple "$obj" "$rel" "$sub"
  done

  # Run assertions
  assert_count=$(yq -r ".tests[$t].assertions // [] | length" "$YAML_PATH")
  for ((a = 0; a < assert_count; a++)); do
    atype=$(yq -r ".tests[$t].assertions[$a].type" "$YAML_PATH")
    if [[ "$atype" != "Check" ]]; then
      echo "  SKIP  unsupported assertion type: $atype"
      continue
    fi
    obj=$(yq -r ".tests[$t].assertions[$a].object" "$YAML_PATH")
    rel=$(yq -r ".tests[$t].assertions[$a].relation" "$YAML_PATH")
    sub=$(yq -r ".tests[$t].assertions[$a].subject" "$YAML_PATH")
    expect_bool=$(yq -r ".tests[$t].assertions[$a].allowed" "$YAML_PATH")
    expect="Denied"
    [[ "$expect_bool" == "true" ]] && expect="Allowed"

    got=$(run_check "$MODEL" "$obj" "$rel" "$sub")
    if [[ "$got" == "$expect" ]]; then
      echo "  PASS  Check($obj, $rel, $sub) = $got"
      pass=$((pass + 1))
    else
      echo "  FAIL  Check($obj, $rel, $sub) expected $expect, got $got"
      fail=$((fail + 1))
    fi
  done
done

echo
echo "== Result: $pass passed, $fail failed =="
[[ $fail -eq 0 ]]
