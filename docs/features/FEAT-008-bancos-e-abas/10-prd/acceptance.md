# @file: docs/features/FEAT-008-bancos-e-abas/10-prd/acceptance.md

# FEAT-008 — Critérios de aceite

## `engine`

- [x] `close_database` fecha a conexão do banco, solta os caches e emite `DatabaseClosed`; os outros bancos da
      conexão seguem abertos; banco desconhecido não emite nada.
- [x] `disconnect` continua fechando todos os bancos da conexão.

## `ui-egui`

- [x] ⌘K com um banco já aberto abre um segundo: a faixa mostra os dois, o segundo ativo, árvore do segundo;
      clicar no primeiro traz a árvore e as abas dele de volta.
- [x] Barra superior mostra o ambiente, a conexão e o banco ativos; "← Conexões" volta à lista sem fechar.
- [x] Clicar numa tabela abre uma aba; clicar noutra abre a segunda; clicar na primeira de novo só a foca;
      × e ⌘W fecham; sem abas, o convite.
- [x] Cada aba tem página, ordenação, seleção e pendência próprias; trocar de aba ou de banco e voltar mantém
      tudo; ordenar e paginar travam só na aba com pendência; o ↻ do banco trava enquanto alguma aba tem
      pendência (esvaziaria as colunas e a chave primária dela).
- [x] Com o Commit rodando numa aba, ×, ⌘W, × do banco e Desconectar não fecham nada (o lote ainda grava).
- [x] Fechar aba, fechar banco ou desconectar com pendência pede confirmação; Cancelar mantém tudo.
- [x] Eventos de dados chegam à aba certa mesmo com duas tabelas carregando ao mesmo tempo.
- [x] Teste `egui_kittest` cobre dois bancos, duas abas, pendência preservada e confirmação ao fechar.

## Portão e CI

- [x] `scripts/quality-gate.sh --full` verde (22/10/2026).
- [ ] Workflow verde (push do Jean).

## Do Jean

- [ ] Abrir dois bancos, duas tabelas em cada, editar numa delas, passear pela faixa e pelas abas, fechar
      uma aba com pendência e ver a confirmação.
