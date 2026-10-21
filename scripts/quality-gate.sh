#!/usr/bin/env bash
# @file: scripts/quality-gate.sh
# Portão de qualidade do hy-dbplus: um script, três chamadores (hook Stop do
# plugin process@hyberica, .githooks/pre-commit e GitHub Actions).
# Adaptado do portão do HyVox (FEAT-219) para um workspace Rust que nasce sem
# passivo: não há linha de base nem catraca; tudo é estrito desde o primeiro
# commit. Regras de "never panic" (unwrap, expect, indexação, unsafe) são lints
# do Clippy declarados em [workspace.lints] do Cargo.toml e reprovam aqui.
#
# Uso:
#   scripts/quality-gate.sh --fast       # fmt + clippy + coerência dos CLAUDE.md (hook Stop)
#   scripts/quality-gate.sh --full       # --fast + testes + cargo-deny (licenças, bans,
#                                        #   fontes) + gitleaks no stage (pre-commit)
#   scripts/quality-gate.sh --full --all --base <ref>
#                                        # CI: --full + advisories (vulnerabilidade publicada
#                                        #   hoje não trava commit local, mas aparece no push)
#                                        #   e gitleaks nos commits desde <ref>; <ref> vazio
#                                        #   ou igual ao HEAD = histórico inteiro
#
# Saída: 0 = verde; 1 = vermelho, com a lista do que falhou no stderr.
#
# Limites conhecidos:
#   - Olha a árvore de trabalho inteira, não o stage: com duas frentes abertas
#     no mesmo diretório, o vermelho pode ser da outra.
#   - Antes do M0 (sem Cargo.toml) as etapas Rust são puladas com aviso.
#
# Compatível com o bash 3.2 do macOS: sem mapfile e sem array associativo.

set -uo pipefail

MODE="fast"
ALL=0
BASE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --fast) MODE="fast" ;;
    --full) MODE="full" ;;
    --all)  ALL=1 ;;
    --base) shift; BASE="${1:-}" ;;
    --baseline)
      echo "portão: este projeto não tem linha de base (nasceu sem passivo); corrija o achado." >&2
      exit 64 ;;
    *) echo "uso: $0 [--fast|--full] [--all] [--base <ref>]" >&2; exit 64 ;;
  esac
  shift
done

ROOT="$(git rev-parse --show-toplevel)" || exit 1
cd "$ROOT" || exit 1

FAILS=""
SUMMARY=""

add_fail() {
  FAILS="${FAILS}  - $1"$'\n'
}

# run <rótulo> <comando...>: roda, guarda a saída e, se falhar, mostra a cauda.
run() {
  local label="$1" out
  shift
  if out="$("$@" 2>&1)"; then
    SUMMARY="${SUMMARY}  ok  $label"$'\n'
  else
    add_fail "$label"
    { echo "--- FALHOU: $label"; printf '%s\n' "$out" | tail -40; } >&2
  fi
}

# tool <nome>: põe em TOOL o caminho da ferramenta instalada por
# scripts/quality-gate-tools.sh. Ausente ou em versão diferente da pedida =
# falha (degrada fechando). Não chamar dentro de $(...): o add_fail se perderia.
TOOLS_BIN="$ROOT/tmp/tools/bin"
TOOL=""
tool() {
  local name="$1" want have
  TOOL=""
  if [ ! -x "$TOOLS_BIN/$name" ]; then
    add_fail "$name não instalado: rode scripts/quality-gate-tools.sh"
    return 1
  fi
  want="$(grep -E "^$(printf '%s' "$name" | tr '[:lower:]-' '[:upper:]_')_VERSION=" scripts/quality-gate-tools.sh | cut -d'"' -f2)"
  have="$(grep -E "^$name " "$TOOLS_BIN/VERSIONS" 2>/dev/null | cut -d' ' -f2)"
  if [ "$want" != "$have" ]; then
    add_fail "$name em versão ${have:-?}, o portão pede $want: rode scripts/quality-gate-tools.sh"
    return 1
  fi
  TOOL="$TOOLS_BIN/$name"
}

# --- o que mudou -------------------------------------------------------------
# REF é a versão "de antes": o HEAD na máquina ou o ponto de partida do push
# no CI (--base). Serve para decidir se o próprio portão mudou e para o
# intervalo do gitleaks; o resto roda sempre, porque o workspace é pequeno.
# REF vazio = sem "antes" (primeiro push, --all sem base, ou base igual ao
# HEAD): tudo conta como mudado e o gitleaks varre o histórico inteiro, em vez
# de comparar HEAD com HEAD e dizer "ok" sem ter olhado nada.
REF=""
if [ -n "$BASE" ]; then
  REF="$(git merge-base "$BASE" HEAD 2>/dev/null)" || { echo "portão: base inválida: $BASE" >&2; exit 1; }
  [ "$REF" = "$(git rev-parse HEAD)" ] && REF=""
elif [ "$ALL" -eq 0 ]; then
  REF="HEAD"
fi
if [ -n "$REF" ] && git rev-parse --verify -q "$REF" >/dev/null; then
  CHANGED="$(git -c core.quotePath=false diff --name-only "$REF" 2>/dev/null; git -c core.quotePath=false ls-files --others --exclude-standard)"
else
  # Sem commit anterior: tudo é novo.
  CHANGED="$(git -c core.quotePath=false ls-files --cached --others --exclude-standard)"
fi

# --- Rust --------------------------------------------------------------------
# --locked: o Cargo.lock é parte do código (supply chain); atualização de
# dependência é decisão, feita de propósito com cargo update.
if [ -f Cargo.toml ]; then
  run "cargo fmt" cargo fmt --all --check
  run "cargo clippy" cargo clippy --workspace --all-targets --locked -- -D warnings
  if [ "$MODE" = "full" ]; then
    run "cargo test" cargo test --workspace --locked
    if tool cargo-deny; then
      if [ "$ALL" -eq 1 ]; then
        run "cargo deny (advisories, licenças, bans, fontes)" "$TOOL" check
      else
        run "cargo deny (licenças, bans, fontes)" "$TOOL" check licenses bans sources
      fi
    fi
  fi
else
  echo "portão: sem Cargo.toml ainda (antes do M0); etapas Rust puladas." >&2
fi

# --- o próprio portão ---------------------------------------------------------
# Mudou o portão (scripts ou pre-commit): sintaxe dos shells e os testes de
# scripts/quality_*_test.py. O workflow só o GitHub valida, no primeiro push.
if printf '%s\n' "$CHANGED" | grep -qE '^(scripts/quality-gate.*\.sh|\.githooks/.*)$'; then
  for f in scripts/quality-gate.sh scripts/quality-gate-tools.sh .githooks/pre-commit; do
    [ -f "$f" ] && run "bash -n $f" bash -n "$f"
  done
fi
# Cada scripts/quality_*_test.py existente (verificador dos CLAUDE.md e o teste do
# próprio portão, que roda este script num repositório temporário sem este teste).
if printf '%s\n' "$CHANGED" | grep -qE '^scripts/quality.*\.(py|sh)$'; then
  for t in scripts/quality_*_test.py; do
    [ -f "$t" ] && run "testes: $t" python3 -m unittest "$t"
  done
fi

# --- CLAUDE.md e regras do Claude Code coerentes ------------------------------
# Um assunto, um dono: sem texto repetido, sem caminho morto, índice completo.
run "coerência dos CLAUDE.md" python3 scripts/quality_claude_docs.py

# --- segredos (só no --full) --------------------------------------------------
# No stage (pre-commit), nos commits do push (CI com base) ou no histórico
# inteiro (CI sem base: primeiro push, disparo manual, execução semanal).
if [ "$MODE" = "full" ] && tool gitleaks; then
  out="$ROOT/tmp/quality-gate-gitleaks.txt"
  mkdir -p "$ROOT/tmp"
  if [ "$ALL" -eq 1 ] && [ -n "$REF" ]; then
    "$TOOL" git --log-opts="$REF..HEAD" --redact --verbose --no-color --no-banner . > "$out" 2>&1
  elif [ "$ALL" -eq 1 ]; then
    "$TOOL" git --redact --verbose --no-color --no-banner . > "$out" 2>&1
  else
    "$TOOL" git --pre-commit --staged --redact --verbose --no-color --no-banner . > "$out" 2>&1
  fi
  if [ $? -ne 0 ]; then
    add_fail "gitleaks: possível segredo (valores ocultados)"
    { echo "--- FALHOU: gitleaks"; grep -E "RuleID|File|Line|Secret|leaks found" "$out" | tail -40; } >&2
  else
    SUMMARY="${SUMMARY}  ok  gitleaks"$'\n'
  fi
fi

# --- veredito -----------------------------------------------------------------
if [ -n "$FAILS" ]; then
  { echo; echo "PORTÃO VERMELHO ($MODE):"; printf '%s' "$FAILS"; } >&2
  exit 1
fi
echo "PORTÃO VERDE ($MODE):"
printf '%s' "$SUMMARY"
exit 0
