# @file: scripts/quality_gate_test.py
# Testes do scripts/quality-gate.sh (FEAT-001). O portao e shell e nasceu com dois
# consertos sem teste: no CI, "--all" sem base (primeiro push) varria HEAD..HEAD e dizia
# "ok" sem olhar commit nenhum, e "--base ''" era tratado como base invalida. Cada teste
# aqui roda o portao de verdade num repositorio git temporario, antes do M0 (sem
# Cargo.toml: as etapas Rust sao puladas, o que tambem e conferido).
#
# O segredo usado nos testes e montado em tempo de execucao: o arquivo-fonte nunca
# contem o token inteiro, senao o proprio gitleaks reprovaria este arquivo no stage.
# Testes que precisam do gitleaks real (tmp/tools/bin, scripts/quality-gate-tools.sh)
# sao pulados quando ele nao esta instalado; os de ferramenta ausente usam uma pasta
# de ferramentas propria.
#
# Uso: python3 -m unittest scripts/quality_gate_test.py   (da raiz do repositorio)

import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
REAL_TOOLS_BIN = os.path.join(REPO, "tmp", "tools", "bin")
# Copiados para o repositorio temporario: o portao, o instalador (de onde o portao le
# as versoes pedidas) e o verificador dos CLAUDE.md com seu teste (o portao os roda
# quando scripts/ muda, e num repositorio novo tudo conta como mudado). Este arquivo
# fica de fora, senao o portao de dentro do teste rodaria o teste de novo.
COPIED = ("quality-gate.sh", "quality-gate-tools.sh", "quality_claude_docs.py", "quality_claude_docs_test.py")
GITLEAKS = os.path.join(REAL_TOOLS_BIN, "gitleaks")


def fake_token():
    # Formato de token pessoal do GitHub (ghp_ + 36 alfanumericos), montado em partes.
    return "gh" + "p_" + "0123456789abcdef" * 2 + "ABCD"


def clean_env():
    """Ambiente sem GIT_*: o pre-commit roda este teste com GIT_INDEX_FILE (e, num
    'git commit -a' ou com caminhos, com caminho absoluto para o indice do commit em
    curso). Herdado, o 'git add' do repositorio temporario reescreveria o indice do
    repositorio real. GIT_AUTHOR_*, GIT_EDITOR e afins saem pelo mesmo motivo."""
    return {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}


class Base(unittest.TestCase):
    def setUp(self):
        self.env = clean_env()
        self.root = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.root)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "teste@example.com")
        self.git("config", "user.name", "teste")
        os.makedirs(os.path.join(self.root, "scripts"))
        for name in COPIED:
            shutil.copy(os.path.join(HERE, name), os.path.join(self.root, "scripts", name))
        self.write("CLAUDE.md", "# raiz\n")
        self.write(".gitignore", "tmp/\n")

    def git(self, *args):
        return subprocess.run(["git", "-C", self.root, *args], capture_output=True, text=True, check=True,
                              env=self.env).stdout.strip()

    def write(self, rel, text):
        path = os.path.join(self.root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-q", "-m", message)
        return self.git("rev-parse", "HEAD")

    def link_real_tools(self):
        os.makedirs(os.path.join(self.root, "tmp", "tools"))
        os.symlink(REAL_TOOLS_BIN, os.path.join(self.root, "tmp", "tools", "bin"))

    def gate(self, *args):
        return subprocess.run(["bash", "scripts/quality-gate.sh", *args], cwd=self.root, capture_output=True, text=True,
                              env=self.env)

    def assert_green(self, result, mode):
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f"PORTÃO VERDE ({mode})", result.stdout)

    def assert_red(self, result, mode, reason):
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(f"PORTÃO VERMELHO ({mode})", result.stderr)
        self.assertIn(reason, result.stderr)


class TestArgumentosEEtapas(Base):
    def test_sem_cargo_toml_pula_rust_e_fica_verde(self):
        self.commit("inicio")
        result = self.gate("--fast")
        self.assert_green(result, "fast")
        self.assertIn("sem Cargo.toml ainda", result.stderr)
        self.assertIn("ok  coerência dos CLAUDE.md", result.stdout)

    def test_argumento_desconhecido_e_baseline_saem_com_64(self):
        self.commit("inicio")
        self.assertEqual(self.gate("--xyz").returncode, 64)
        result = self.gate("--baseline")
        self.assertEqual(result.returncode, 64)
        self.assertIn("não tem linha de base", result.stderr)

    def test_base_invalida_reprova(self):
        self.commit("inicio")
        result = self.gate("--full", "--all", "--base", "nao-existe")
        self.assertEqual(result.returncode, 1)
        self.assertIn("base inválida", result.stderr)

    def test_ferramenta_ausente_ou_em_outra_versao_reprova(self):
        self.commit("inicio")
        bin_dir = os.path.join(self.root, "tmp", "tools", "bin")
        os.makedirs(bin_dir)
        self.assert_red(self.gate("--full"), "full", "gitleaks não instalado")
        self.write("tmp/tools/bin/gitleaks", "#!/bin/sh\nexit 0\n")
        os.chmod(os.path.join(bin_dir, "gitleaks"), 0o755)
        self.write("tmp/tools/bin/VERSIONS", "gitleaks v0.0.0\n")
        self.assert_red(self.gate("--full"), "full", "gitleaks em versão v0.0.0")

    def test_nao_herda_o_indice_do_commit_em_curso(self):
        # Simula o pre-commit de 'git commit -a': GIT_INDEX_FILE absoluto para o indice de
        # outro repositorio (o "real"). O teste nao pode tocar nele.
        real = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, real)
        env = clean_env()
        subprocess.run(["git", "-C", real, "init", "-q", "-b", "main"], check=True, env=env)
        with open(os.path.join(real, "a.txt"), "w", encoding="utf-8") as fh:
            fh.write("a\n")
        subprocess.run(["git", "-C", real, "add", "a.txt"], check=True, env=env)
        ls = lambda: subprocess.run(["git", "-C", real, "ls-files"], capture_output=True, text=True, check=True,
                                    env=env).stdout
        self.assertEqual(ls(), "a.txt\n")
        os.environ["GIT_INDEX_FILE"] = os.path.join(real, ".git", "index")
        self.addCleanup(os.environ.pop, "GIT_INDEX_FILE", None)
        self.env = clean_env()
        self.commit("inicio")
        self.assert_green(self.gate("--fast"), "fast")
        self.assertEqual(ls(), "a.txt\n")


@unittest.skipUnless(os.access(GITLEAKS, os.X_OK), "gitleaks não instalado em tmp/tools/bin (scripts/quality-gate-tools.sh)")
class TestGitleaks(Base):
    """Historico: a (limpo) -> b (segredo) -> c (limpo). O segredo nao esta no diff de c
    nem no stage: so aparece para quem olha o historico ou o intervalo que inclui b."""

    def setUp(self):
        super().setUp()
        self.link_real_tools()
        self.a = self.commit("a: limpo")
        self.write("config.txt", f"token={fake_token()}\n")
        self.b = self.commit("b: segredo")
        self.write("README.md", "# leia\n")
        self.c = self.commit("c: limpo")

    def test_all_sem_base_varre_o_historico_inteiro(self):
        # Antes do conserto: HEAD..HEAD, zero commits, "ok".
        self.assert_red(self.gate("--full", "--all"), "full", "gitleaks: possível segredo")

    def test_base_vazia_equivale_ao_historico_inteiro(self):
        # Antes do conserto: 'git merge-base "" HEAD' falhava e o portao dizia "base inválida".
        result = self.gate("--full", "--all", "--base", "")
        self.assert_red(result, "full", "gitleaks: possível segredo")
        self.assertNotIn("base inválida", result.stderr)

    def test_base_igual_ao_head_varre_o_historico_inteiro(self):
        self.assert_red(self.gate("--full", "--all", "--base", self.c), "full", "gitleaks: possível segredo")

    def test_base_limita_o_intervalo_dos_commits(self):
        # Base em a: o intervalo a..HEAD inclui b. Base em b: so c, limpo.
        self.assert_red(self.gate("--full", "--all", "--base", self.a), "full", "gitleaks: possível segredo")
        self.assert_green(self.gate("--full", "--all", "--base", self.b), "full")

    def test_full_sem_all_olha_so_o_stage(self):
        # Pre-commit: o historico nao interessa, so o que vai entrar no commit.
        self.assert_green(self.gate("--full"), "full")
        self.write("novo.txt", f"senha={fake_token()}\n")
        self.git("add", "novo.txt")
        self.assert_red(self.gate("--full"), "full", "gitleaks: possível segredo")
        # --fast (hook Stop) nao roda o gitleaks: o segredo so e barrado no commit.
        self.assert_green(self.gate("--fast"), "fast")



@unittest.skipUnless(os.access(GITLEAKS, os.X_OK), "gitleaks não instalado em tmp/tools/bin (scripts/quality-gate-tools.sh)")
class TestGitleaksHistoricoLimpo(Base):
    """Sem intervalo o gitleaks varre todas as refs, nao so a branch atual: o historico
    limpo precisa de um repositorio proprio, e nao de uma branch nova no de cima."""

    def test_historico_limpo_sem_base_fica_verde(self):
        self.link_real_tools()
        self.commit("a: limpo")
        self.write("README.md", "# leia\n")
        self.commit("b: limpo")
        self.assert_green(self.gate("--full", "--all"), "full")
        self.assert_green(self.gate("--full", "--all", "--base", ""), "full")


if __name__ == "__main__":
    sys.exit(unittest.main())
