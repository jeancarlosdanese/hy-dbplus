# @file: docs/features/FEAT-016-versao-e-instaladores/10-prd/acceptance.md

# Critérios de aceite: FEAT-016

Sem tela nova e sem contas: o que "não pode acontecer" aqui é versão errada, pacote com a versão trocada, Release
sem um dos pacotes e segredo no build.

```gherkin
Cenário: subir a versão
  Dado o VERSION e o Cargo.toml em 0.1.0
  Quando roda `cargo xtask version 0.2.0`
  Então o VERSION, o [workspace.package] do Cargo.toml e as crates do workspace no Cargo.lock ficam em 0.2.0
  E nenhuma outra linha muda (dependência de terceiros em 0.1.0 continua 0.1.0)
  E `cargo metadata --locked --offline` aceita o lock (o comando confere e, se falhar, devolve os três arquivos)

Cenário: versão que não sobe ou inválida
  Dado o VERSION em 0.2.0
  Quando roda `cargo xtask version 0.2.0`, `0.1.9`, `0.3`, `v0.3.0` ou `0.3.0-beta`
  Então o comando falha com a mensagem do motivo
  E nenhum arquivo muda

Cenário: versão fora de compasso
  Dado o VERSION em 0.2.0 e o Cargo.toml em 0.1.0
  Quando roda o portão completo
  Então o teste do xtask falha dizendo os dois valores

Cenário: entrega com versão
  Dado o process.json com version e bump
  Quando o Jean manda "entrega" de uma FEAT
  Então, depois do merge, a IA roda o bump com a versão minor seguinte
  E commita só a troca da versão, aceita pela trava
  E cria a tag v<versão> nesse commit

Cenário: pacotes do Linux
  Dado um Ubuntu com as dependências de sistema
  Quando roda `cargo xtask bundle`
  Então saem hy-dbplus-<versão>-linux-x86_64.deb e .tar.gz em target/bundle/
  E o .deb instala /usr/bin/hy-dbplus, o .desktop em /usr/share/applications e o ícone em
    /usr/share/icons/hicolor/256x256/apps
  E o campo Version do .deb é a versão do workspace

Cenário: pacote do Windows
  Dado um Windows
  Quando roda `cargo xtask bundle`
  Então sai hy-dbplus-<versão>-windows-x86_64.zip com o hy-dbplus.exe e a licença
  E o hy-dbplus.exe é um programa gráfico (abre sem janela de console)

Cenário: macOS sem regressão
  Dado um Mac com o Xcode
  Quando roda `cargo xtask bundle --universal`
  Então sai hy-dbplus-<versão>-macos-universal.dmg com o .app assinado ad hoc, como na FEAT-014

Cenário: Release pela tag
  Dado a tag v0.2.0 no commit cujo VERSION é 0.2.0
  Quando o Jean faz o push da tag
  Então o release.yml monta os quatro pacotes e o SHA256SUMS
  E cria a Release v0.2.0 em rascunho com os cinco arquivos

Cenário: tag que não é a versão
  Dado a tag v0.3.0 num commit cujo VERSION é 0.2.0
  Quando o push da tag dispara o release.yml
  Então o workflow falha antes de montar qualquer pacote
  E nenhuma Release é criada

Cenário: nada de segredo
  Quando o release.yml roda
  Então ele usa só o GITHUB_TOKEN do próprio run, com escrita restrita ao job que cria a Release
  E nenhum pacote leva arquivo de conexão, preferência ou senha
```

## Checklist

- [x] AC-001: `cargo xtask version` troca a versão nos três arquivos e só nas linhas da versão. Verificar: teste.
- [x] AC-002: versão inválida ou que não sobe é recusada sem mudar arquivo. Verificar: teste.
- [x] AC-003: `VERSION` e `Cargo.toml` diferentes reprovam o portão. Verificar: teste.
- [ ] AC-004 (motivo: a leitura e a simulação estão feitas; a prova pela trava é a própria entrega): `process.json`
      com `version` e `bump`, e a regra de versão no `CLAUDE.md`. Verificar: leitura; a
      trava na entrega desta FEAT (merge e a tag `v0.2.0` no commit de merge, porque a 0.2.0 subiu na branch) e,
      por simulação com as funções da própria trava, o commit da versão na entrega seguinte (a prova real é a
      dessa entrega).
- [x] AC-005: nomes dos pacotes, `control` do `.deb`, `.desktop` e árvore dos pacotes. Verificar: teste (em
      qualquer sistema).
- [ ] AC-006 (motivo: só o CI monta; roda no push da `main` e da tag `v0.2.0` pelo Jean, depois da entrega): `.deb` e `.tar.gz` montados no Ubuntu. Verificar: CI (o `release.yml` também roda no
      push da `main` e em pull request que tocam `xtask/`, `packaging/`, `apps/hy-dbplus/`, `apps/hy-dbplus-cli/`, `VERSION`, `Cargo.toml`, `Cargo.lock` ou o próprio workflow, só montando
      os pacotes, sem Release; o log traz `dpkg-deb --info`, `--contents` e o dono 0/0 do `.tar.gz`).
- [ ] AC-007 (motivo: só o CI monta; roda no push da `main` e da tag `v0.2.0` pelo Jean, depois da entrega): `.zip` montado no Windows e o `.exe` com subsistema gráfico. Verificar: CI (o mesmo run) e
      teste do atributo.
- [x] AC-008: `.dmg` do macOS sem regressão. Verificar: `cargo xtask bundle` no Mac (arm64, feito; o universal
      monta no CI, porque este Mac não tem o target x86_64).
- [x] AC-009: tag diferente da versão faz o workflow falhar antes dos pacotes. Verificar: teste da conferência (o
      passo chama `cargo xtask version --check <tag>`).
- [ ] AC-010 (motivo: depende do push da tag `v0.2.0`, que é do Jean): Release em rascunho com os cinco arquivos no push da tag. Verificar: push da `v0.2.0` pelo Jean.
- [x] AC-011: `GITHUB_TOKEN` com `contents: write` só no job da Release; nenhum segredo no workflow. Verificar:
      revisor.
