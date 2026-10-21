# @file: scripts/quality_claude_docs.py
# Coerencia dos CLAUDE.md e das regras do Claude Code (FEAT-219 fase 7). O
# CLAUDE.md da raiz carrega sempre; o de cada app (apps/*/CLAUDE.md) e as regras
# (.claude/rules/*.md, com "paths:") carregam quando o trabalho chega la. Um
# complementa o outro, entao nada pode se repetir nem apontar para o que nao
# existe mais (o CLAUDE.md antigo citava arquivo movido e funcao renomeada).
#
# Confere:
#   - tamanho: raiz <= 200 linhas, app <= 300, regra <= 120;
#   - todo CLAUDE.md de app e toda regra apontam para o da raiz, e a raiz lista
#     todos (indice completo, nada orfao);
#   - linha repetida (>= 60 caracteres, fora de bloco de codigo) em dois arquivos:
#     "um assunto, um dono"; repeticao com outras palavras fica para o revisor;
#   - caminho citado (link ou `crase`) existe; glob do "paths:" casa com algum
#     arquivo do repositorio (regra que nunca carrega e regra morta).
#
# Uso: python3 scripts/quality_claude_docs.py   (da raiz do repositorio)
# Saida: 0 = ok; 1 = problemas no stderr.

import glob
import os
import re
import subprocess
import sys

LIMITS = {"root": 200, "app": 300, "rule": 120}
MIN_DUP = 60
# Copiado de plugins/process/scripts/ do hyberica-claude-kit (o CI nao alcanca o plugin);
# so as raizes mudaram: workspace Rust (crates/, apps/, xtask/) em vez de Go + Next.
REPO_ROOTS = ("apps/", "crates/", "xtask/", "scripts/", "docs/", ".claude/", ".githooks/", ".github/",
              "packaging/", "backlog/")
APP_ROOTS = ("src/", "tests/", "examples/", "benches/", "scripts/")
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
TICK_RE = re.compile(r"`([^`\s]+)`")


def glob_regex(pattern):
    """Glob com ** (qualquer profundidade) e * (dentro de um nivel) para regex."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pattern.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pattern[i] == "*":
            out, i = out + "[^/]*", i + 1
        elif pattern[i] == "?":
            out, i = out + "[^/]", i + 1
        else:
            out, i = out + re.escape(pattern[i]), i + 1
    return re.compile("^" + out + "$")


def rule_globs(text):
    """Globs do bloco "paths:" do cabecalho YAML de uma regra."""
    if not text.startswith("---"):
        return []
    end = text.find("\n---", 3)
    header = text[3:end] if end > 0 else ""
    globs, inside = [], False
    for line in header.split("\n"):
        if re.match(r"^paths\s*:", line):
            inside = True
            inline = line.split(":", 1)[1].strip()
            if inline:
                globs.append(inline.strip("\"'"))
            continue
        m = re.match(r"^\s+-\s+(.+)$", line)
        if inside and m:
            globs.append(m.group(1).strip().strip("\"'"))
        elif line.strip():
            inside = False
    return globs


def glob_matches(pattern, path):
    return bool(glob_regex(pattern).match(path))


def docs(root):
    found = [("root", "CLAUDE.md")]
    found += [("app", os.path.relpath(p, root)) for p in sorted(glob.glob(os.path.join(root, "apps", "*", "CLAUDE.md")))]
    found += [("rule", os.path.relpath(p, root)) for p in sorted(glob.glob(os.path.join(root, ".claude", "rules", "*.md")))]
    return [(k, p) for k, p in found if os.path.exists(os.path.join(root, p))]


def normalize(line):
    line = re.sub(r"^\s*(?:[-*>|]|\d+\.)\s*", "", line.strip())
    line = re.sub(r"[`*_]", "", line)
    return re.sub(r"\s+", " ", line).strip().lower()


def exists(root, rel):
    if any(c in rel for c in "*?"):
        return bool(glob.glob(os.path.join(root, rel), recursive=True))
    # "docs/changes/0090" cita o documento pelo numero: vale como prefixo do nome.
    return os.path.exists(os.path.join(root, rel)) or bool(glob.glob(os.path.join(root, rel) + "_*"))


def check_paths(root, kind, rel, text):
    problems, base = [], os.path.dirname(rel)
    for target in LINK_RE.findall(text):
        if re.match(r"^(https?:|mailto:|#)", target):
            continue
        path = os.path.normpath(os.path.join(base, target.split("#")[0]))
        if not exists(root, path):
            problems.append(f"{rel}: link para {target} que não existe")
    for token in TICK_RE.findall(text):
        token = re.sub(r"(:\d+|#L\d+)$", "", token.rstrip(".,;:)"))
        while token.startswith("./"):
            token = token[2:]
        if "/" not in token or re.search(r"[<>{}$|=]|\.\.\.|NNN|^/|^https?:", token):
            continue
        # Num CLAUDE.md de app, o caminho pode ser do app (scripts/ do wa-gateway) ou da raiz.
        candidates = []
        if kind == "app" and token.startswith(APP_ROOTS):
            candidates.append(os.path.join(base, token))
        if token.startswith(REPO_ROOTS):
            candidates.append(token)
        if not candidates:
            continue  # caminho relativo a outro lugar: nao da para conferir sem adivinhar
        if not any(exists(root, c.rstrip("/")) for c in candidates):
            problems.append(f"{rel}: cita `{token}`, que não existe")
    return problems


def check(root, tracked):
    problems, seen = [], {}
    found = docs(root)
    root_text = open(os.path.join(root, "CLAUDE.md"), encoding="utf-8").read() if found else ""
    for kind, rel in found:
        with open(os.path.join(root, rel), encoding="utf-8") as fh:
            text = fh.read()
        lines = text.split("\n")
        if len(lines) > LIMITS[kind]:
            problems.append(f"{rel}: {len(lines)} linhas, limite {LIMITS[kind]} (mova o detalhe para o app ou para uma regra)")
        if kind != "root":
            back = os.path.relpath("CLAUDE.md", os.path.dirname(rel))
            if f"]({back})" not in text:
                problems.append(f"{rel}: não aponta para o CLAUDE.md da raiz ({back})")
            if f"]({rel})" not in root_text:
                problems.append(f"CLAUDE.md: não lista {rel} (índice incompleto)")
        if kind == "rule":
            globs = rule_globs(text)
            if not globs:
                problems.append(f"{rel}: sem \"paths:\" no cabeçalho (regra sem caminho carrega sempre)")
            for g in globs:
                if not any(glob_matches(g, f) for f in tracked):
                    problems.append(f"{rel}: o caminho \"{g}\" não casa com nenhum arquivo (regra que nunca carrega)")
        problems += check_paths(root, kind, rel, text)
        in_code = False
        for n, line in enumerate(lines, 1):
            if line.strip().startswith("```"):
                in_code = not in_code
                continue
            norm = normalize(line)
            # A linha que aponta para a raiz e obrigatoria em todo arquivo: repeti-la e o esperado.
            if in_code or len(norm) < MIN_DUP or set(norm) <= set("-|: ") or "](../../claude.md)" in norm:
                continue
            seen.setdefault(norm, []).append(f"{rel}:{n}")
    for norm, where in seen.items():
        files = {w.rsplit(":", 1)[0] for w in where}
        if len(files) > 1:
            problems.append(f"texto repetido em {', '.join(where)}: um assunto, um dono (deixe num lugar e aponte para ele)")
    return problems


def main():
    root = os.getcwd()
    tracked = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard"], capture_output=True,
                             text=True, check=True).stdout.split("\n")
    problems = check(root, [t for t in tracked if t])
    if problems:
        print("CLAUDE.md e regras do Claude Code:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
