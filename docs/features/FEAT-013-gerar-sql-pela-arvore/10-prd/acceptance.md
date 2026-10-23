# @file: docs/features/FEAT-013-gerar-sql-pela-arvore/10-prd/acceptance.md

# FEAT-013 — Critérios de aceite

## `catalog::generate`

- [ ] Cada modelo (SELECT, SELECT count, INSERT, UPDATE, DELETE, TRUNCATE, DROP CASCADE) sai com o
      identificador qualificado, aspas só onde precisa, e termina em `;`.
- [ ] INSERT pula coluna de identidade `ALWAYS`, põe `DEFAULT` em coluna com default e um valor de exemplo
      pelo tipo nas outras; UPDATE e DELETE usam a chave primária no `WHERE` e avisam em comentário quando
      não há chave.
- [ ] View e view materializada só recebem SELECT, SELECT count, CREATE e o `DROP` do tipo certo.

## `ui-egui`

- [ ] Botão direito numa tabela da árvore abre o menu; um clique num item põe o SQL na aba SQL ativa (ou
      na última do banco, ou numa nova) no fim do buffer, com linha em branco antes quando havia texto,
      cursor no statement novo, aba ativa e editor com foco.
- [ ] Tabela sem colunas carregadas: o menu pede ao engine e o SQL entra quando as colunas chegam; CREATE
      espera a estrutura; falha do catálogo vira aviso na aba e não deixa pedido pendurado.
- [ ] Nenhum item executa SQL.
- [ ] Teste `egui_kittest` cobre menu → aba nova, segundo modelo na mesma aba, colunas sob demanda e CREATE.

## Portão e CI

- [ ] `scripts/quality-gate.sh --full` verde; [ ] workflow verde (push do Jean).

## Do Jean

- [ ] Botão direito numa tabela com chave primária, INSERT, UPDATE e DELETE: conferir colunas, valores de
      exemplo e `WHERE`; DROP CASCADE e TRUNCATE aparecem no editor e não rodam; CREATE bate com a aba
      Structure; numa view só os itens de view.
