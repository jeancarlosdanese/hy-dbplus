#!/usr/bin/env bash
# @file: scripts/quality-gate-tools.sh
# Instala, com versão fixa, as ferramentas do portão de qualidade que não vêm
# com o toolchain Rust. Ficam em tmp/tools/bin (ignorado pelo git); o
# scripts/quality-gate.sh procura ali e confere a versão. Rodar uma vez por
# máquina e de novo quando as versões abaixo mudarem.
#
# Por que cada uma:
#   cargo-deny: licença de cada dependência contra a lista permitida (deny.toml),
#               vulnerabilidade publicada (RustSec), crate proibida e fonte
#               desconhecida (supply chain).
#   gitleaks:   chave ou senha entrando num commit.
# fmt, clippy e test vêm com o rustup e não precisam ficar aqui.

set -euo pipefail

CARGO_DENY_VERSION="0.20.2"
GITLEAKS_VERSION="v8.30.1"

ROOT="$(git rev-parse --show-toplevel)"
BIN="$ROOT/tmp/tools/bin"
mkdir -p "$BIN"

echo "instalando cargo-deny ($CARGO_DENY_VERSION)"
cargo install cargo-deny --version "$CARGO_DENY_VERSION" --locked --root "$ROOT/tmp/tools"

# gitleaks é escrito em Go: na máquina de desenvolvimento e no CI o Go está
# disponível (como no HyVox). Sem Go, instalar pelo brew e copiar para $BIN.
if command -v go >/dev/null 2>&1; then
  echo "instalando gitleaks ($GITLEAKS_VERSION)"
  GOBIN="$BIN" GOFLAGS=-mod=mod go install "github.com/zricethezav/gitleaks/v8@$GITLEAKS_VERSION"
else
  echo "go não encontrado: instale o gitleaks $GITLEAKS_VERSION por outro meio e copie para $BIN" >&2
  exit 1
fi

# A versão instalada fica registrada: o portão avisa quando o script pede outra.
{
  echo "cargo-deny $CARGO_DENY_VERSION"
  echo "gitleaks $GITLEAKS_VERSION"
} > "$BIN/VERSIONS"
echo "ferramentas em $BIN"
