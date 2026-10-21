# @file: scripts/quality_claude_docs_test.py
# Testes da coerencia dos CLAUDE.md e das regras (FEAT-219 fase 7).
# Copia do kit; so a fixture mudou de Go + Next (lib/*.ts) para Rust (src/*.rs),
# porque as raizes de app do verificador aqui sao src/, tests/, examples/, benches/.
# Rodar: python3 -m unittest scripts/quality_claude_docs_test.py

import os
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import quality_claude_docs as qc  # noqa: E402

ROOT_OK = "# raiz\n\nVer [app](apps/web/CLAUDE.md) e [regra](.claude/rules/r.md).\n"
APP_OK = "# app\n\nComplementa o [CLAUDE.md da raiz](../../CLAUDE.md).\n\nCódigo em `src/util.rs`.\n"
RULE_OK = "---\npaths:\n  - \"apps/web/src/**\"\n---\n\n# regra\n\nComplementa o [CLAUDE.md da raiz](../../CLAUDE.md).\n"


class Base(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.root)
        self.write("CLAUDE.md", ROOT_OK)
        self.write("apps/web/CLAUDE.md", APP_OK)
        self.write(".claude/rules/r.md", RULE_OK)
        self.write("apps/web/src/util.rs", "x")

    def write(self, rel, text):
        path = os.path.join(self.root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)

    def problems(self):
        tracked = []
        for d, _, files in os.walk(self.root):
            tracked += [os.path.relpath(os.path.join(d, f), self.root) for f in files]
        return qc.check(self.root, tracked)


class TestCoerencia(Base):
    def test_conjunto_coerente_passa(self):
        self.assertEqual(self.problems(), [])

    def test_texto_repetido_reprova(self):
        frase = "Nunca rodar migration nem reiniciar serviço; o ambiente de dev já está rodando sempre."
        self.write("CLAUDE.md", ROOT_OK + f"\n- {frase}\n")
        self.write("apps/web/CLAUDE.md", APP_OK + f"\n1. **{frase}**\n")
        self.assertTrue(any("texto repetido" in p for p in self.problems()))

    def test_repeticao_dentro_de_bloco_de_codigo_nao_conta(self):
        linha = "AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION, AWS_S3_BUCKET, OUTRA_VARIAVEL"
        self.write("CLAUDE.md", ROOT_OK + f"\n```\n{linha}\n```\n")
        self.write("apps/web/CLAUDE.md", APP_OK + f"\n```\n{linha}\n```\n")
        self.assertEqual(self.problems(), [])

    def test_caminho_citado_que_nao_existe(self):
        self.write("apps/web/CLAUDE.md", APP_OK + "\nVer `src/sumiu.rs` e [doc](../../docs/x.md).\n")
        ps = self.problems()
        self.assertTrue(any("src/sumiu.rs" in p for p in ps))
        self.assertTrue(any("docs/x.md" in p for p in ps))

    def test_numero_de_documento_vale_como_prefixo(self):
        self.write("docs/changes/0090_correcao.md", "x")
        self.write("apps/web/CLAUDE.md", APP_OK + "\nVer `docs/changes/0090` e `docs/FEAT-NNN-*/`.\n")
        self.assertEqual(self.problems(), [])

    def test_linha_que_aponta_para_a_raiz_pode_repetir(self):
        linha = "Complementa o [CLAUDE.md da raiz](../../CLAUDE.md), que vale sempre e tem as regras invioláveis."
        self.write("apps/web/CLAUDE.md", f"# app\n\n{linha}\n\nCódigo em `src/util.rs`.\n")
        self.write(".claude/rules/r.md", RULE_OK + f"\n{linha}\n")
        self.assertEqual(self.problems(), [])

    def test_script_do_proprio_app_e_caminho_com_ponto(self):
        self.write("apps/web/scripts/build.sh", "x")
        self.write("apps/web/CLAUDE.md", APP_OK + "\nRodar `scripts/build.sh` e `./scripts/build.sh`.\n")
        self.assertEqual(self.problems(), [])
        self.write("apps/web/CLAUDE.md", APP_OK + "\nRodar `./scripts/sumiu.sh`.\n")
        self.assertTrue(any("scripts/sumiu.sh" in p for p in self.problems()))

    def test_regra_que_nunca_carrega(self):
        self.write(".claude/rules/r.md", RULE_OK.replace("apps/web/src/**", "apps/nada/**"))
        self.assertTrue(any("nunca carrega" in p for p in self.problems()))

    def test_regra_sem_paths(self):
        self.write(".claude/rules/r.md", "# regra\n\n[raiz](../../CLAUDE.md)\n")
        self.assertTrue(any("sem \"paths:\"" in p for p in self.problems()))

    def test_app_sem_apontar_para_a_raiz(self):
        self.write("apps/web/CLAUDE.md", "# app\n")
        self.assertTrue(any("não aponta para o CLAUDE.md da raiz" in p for p in self.problems()))

    def test_raiz_sem_listar_o_app(self):
        self.write("CLAUDE.md", "# raiz\n\n[regra](.claude/rules/r.md)\n")
        self.assertTrue(any("não lista apps/web/CLAUDE.md" in p for p in self.problems()))

    def test_limite_de_linhas(self):
        self.write("CLAUDE.md", ROOT_OK + "x\n" * 201)
        self.assertTrue(any("limite 200" in p for p in self.problems()))


class TestGlob(unittest.TestCase):
    def test_globs(self):
        self.assertTrue(qc.glob_matches("apps/x/**", "apps/x/a/b/c.go"))
        self.assertTrue(qc.glob_matches("apps/x/**/*.ts", "apps/x/a.ts"))
        self.assertTrue(qc.glob_matches("apps/x/internal/service/*ai*.go", "apps/x/internal/service/ai_usage.go"))
        self.assertFalse(qc.glob_matches("apps/x/internal/service/*ai*.go", "apps/x/internal/service/sub/ai.go"))
        self.assertEqual(qc.rule_globs('---\npaths:\n  - "a/**"\n  - b/*.ts\n---\n'), ["a/**", "b/*.ts"])


if __name__ == "__main__":
    unittest.main()
