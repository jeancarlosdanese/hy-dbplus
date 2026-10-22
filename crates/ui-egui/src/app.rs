//! O app egui: estado, roteamento entre as telas e leitura dos eventos do engine a cada frame.
//! Genérico no driver para o teste de interação rodar com um engine falso; o binário usa
//! [`PgApp`].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use hy_dbplus_engine::{
    ChangeSet, Column, ConnectionId, ConnectionStore, DatabaseInfo, Driver, Event, Function, Page,
    PostgresDriver, RowUpdate, Schema, SecretStore, Sessions, Table, TableStats, TextRows,
    preview_sql,
};

use crate::connections::{ConnectionForm, Connections};
use crate::theme;

/// O app com o driver PostgreSQL, como o binário o cria.
pub type PgApp = App<PostgresDriver>;

/// O que o app recebe pronto do binário: o engine é dele, a UI só usa.
pub struct Deps<D: Driver> {
    pub store: ConnectionStore,
    pub secrets: Box<dyn SecretStore>,
    pub sessions: Sessions<D>,
    pub events: Receiver<Event>,
}

/// Estado de uma conexão do ponto de vista da UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Idle,
    Connecting,
    Connected {
        server_version: String,
        databases: Vec<DatabaseInfo>,
    },
    Failed(String),
}

/// O formato do painel de registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelMode {
    #[default]
    Form,
    Json,
}

/// Qual tela está na frente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Connections,
    Workspace { connection: ConnectionId },
}

/// O banco selecionado numa conexão e o catálogo que a UI já recebeu (tela 3.3).
#[derive(Debug, Default)]
pub struct Workspace {
    pub database: Option<String>,
    pub schemas: Vec<Schema>,
    pub current_schema: Option<String>,
    /// Tabelas e funções por schema, como chegaram do engine.
    pub objects: HashMap<String, (Vec<Table>, Vec<Function>)>,
    pub columns: HashMap<(String, String), Vec<Column>>,
    /// Tabelas abertas na árvore, por (schema, tabela).
    pub expanded: HashSet<(String, String)>,
    pub loading: bool,
    pub error: Option<String>,
    pub search: String,
    /// A tabela aberta na aba Data, se houver.
    pub data: Option<DataTab>,
}

impl Workspace {
    /// As colunas do catálogo (nome, tipo) na ordem das colunas de uma página de dados, casadas pelo
    /// nome; `None` onde o cache não tem a coluna. Cabeçalho do grid, Form e JSON usam esta busca.
    pub fn catalog_columns(
        &self,
        schema: &str,
        table: &str,
        names: &[String],
    ) -> Vec<Option<&Column>> {
        let cache = self.columns.get(&(schema.to_owned(), table.to_owned()));
        names
            .iter()
            .map(|name| cache.and_then(|c| c.iter().find(|c| &c.name == name)))
            .collect()
    }
}

/// Uma célula do grid em edição (FEAT-007): posição na página e o texto digitado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellEdit {
    pub row: usize,
    pub col: usize,
    pub text: String,
    /// O valor de onde a edição partiu (pendente ou da página): texto vazio sobre NULL continua NULL.
    pub original: Option<String>,
    /// Pede o foco do campo uma vez, ao abrir.
    pub focus_pending: bool,
}

/// A caixa de Commit (FEAT-007): o SQL que vai rodar, as contagens, se já está rodando e o erro
/// do servidor (statement ou contagem: desfeito; conexão caída no COMMIT: a mensagem diz; as
/// pendências continuam).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitDialog {
    pub sql: Vec<String>,
    pub updates: usize,
    pub deletes: usize,
    pub running: bool,
    pub error: Option<String>,
    /// O statement que falhou (a partir de 1), destacado no SQL.
    pub failed: Option<usize>,
}

/// Aviso quando uma página nova chega com edição pendente (não deveria acontecer: a navegação trava).
pub const DROPPED_HINT: &str =
    "A página foi recarregada e as alterações pendentes foram descartadas";

/// A aba Data de uma tabela (tela 3.4): página atual, ordenação, contagem, tamanhos, seleção e as
/// edições pendentes (FEAT-007).
#[derive(Debug, Clone)]
pub struct DataTab {
    pub schema: String,
    pub table: String,
    /// A página confirmada (a das `rows`); `requested` é a pedida e ainda não chegada.
    pub page: Page,
    pub requested: Option<Page>,
    /// `Arc`: a aba é clonada a cada frame para desenhar; as linhas não.
    pub rows: Option<Arc<TextRows>>,
    pub stats: Option<TableStats>,
    /// Linhas selecionadas (índices da página); `anchor` é a última clicada, para ⇧ estender.
    pub selected_rows: BTreeSet<usize>,
    pub anchor: Option<usize>,
    /// Campos do Form abertos por "mostrar tudo".
    pub expanded_fields: HashSet<String>,
    /// Edições pendentes por linha da página: coluna → valor novo (`None` é NULL). Nada vai ao
    /// banco até o Commit; valor igual ao da página não fica pendente.
    pub pending: BTreeMap<usize, BTreeMap<String, Option<String>>>,
    /// Linhas da página marcadas para apagar.
    pub deleted: BTreeSet<usize>,
    pub editing: Option<CellEdit>,
    pub commit: Option<CommitDialog>,
    pub loading: bool,
    pub error: Option<String>,
}

impl DataTab {
    pub fn new(schema: &str, table: &str) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            page: Page::default(),
            requested: Some(Page::default()),
            rows: None,
            stats: None,
            selected_rows: BTreeSet::new(),
            anchor: None,
            expanded_fields: HashSet::new(),
            pending: BTreeMap::new(),
            deleted: BTreeSet::new(),
            editing: None,
            commit: None,
            loading: true,
            error: None,
        }
    }

    /// Há edição ou exclusão pendente: ordenar, paginar, recarregar e trocar de tabela ficam
    /// travados até o Commit ou o Descartar.
    pub fn has_changes(&self) -> bool {
        !self.pending.is_empty() || !self.deleted.is_empty()
    }

    /// (linhas alteradas que não estão marcadas para apagar, linhas a apagar).
    pub fn change_counts(&self) -> (usize, usize) {
        let updates = self
            .pending
            .keys()
            .filter(|row| !self.deleted.contains(row))
            .count();
        (updates, self.deleted.len())
    }

    /// O valor que o grid e o Form mostram para a célula: o pendente, senão o da página; `None`
    /// se a célula não existe.
    pub fn cell_value(&self, row: usize, col: usize) -> Option<Option<&str>> {
        let rows = self.rows.as_ref()?;
        let name = rows.columns.get(col)?;
        if let Some(value) = self.pending.get(&row).and_then(|m| m.get(name)) {
            return Some(value.as_deref());
        }
        Some(rows.rows.get(row)?.get(col)?.as_deref())
    }

    pub fn is_pending(&self, row: usize, column: &str) -> bool {
        self.pending
            .get(&row)
            .is_some_and(|m| m.contains_key(column))
    }

    /// Guarda o valor novo da célula; igual ao da página, a pendência some.
    pub fn set_cell(&mut self, row: usize, col: usize, value: Option<String>) {
        let Some(rows) = self.rows.as_ref() else {
            return;
        };
        let Some(name) = rows.columns.get(col).cloned() else {
            return;
        };
        let Some(original) = rows.rows.get(row).and_then(|r| r.get(col)) else {
            return;
        };
        if &value == original {
            self.revert_cell(row, &name);
            return;
        }
        self.pending.entry(row).or_default().insert(name, value);
    }

    pub fn revert_cell(&mut self, row: usize, column: &str) {
        if let Some(map) = self.pending.get_mut(&row) {
            map.remove(column);
            if map.is_empty() {
                self.pending.remove(&row);
            }
        }
    }

    pub fn clear_changes(&mut self) {
        self.pending.clear();
        self.deleted.clear();
        self.editing = None;
    }

    /// A página pedida chegou: troca as linhas e limpa a seleção. Pendência, edição ou caixa sobre
    /// a página antiga são descartadas com aviso (os índices apontariam outras linhas); devolve
    /// se descartou algo.
    pub fn accept_page(&mut self, page: Page, rows: TextRows) -> bool {
        let dropped = self.has_changes() || self.editing.is_some() || self.commit.is_some();
        self.page = page;
        self.requested = None;
        self.rows = Some(Arc::new(rows));
        self.loading = false;
        self.error = dropped.then(|| DROPPED_HINT.to_owned());
        self.selected_rows.clear();
        self.anchor = None;
        self.clear_changes();
        self.commit = None;
        dropped
    }

    /// O lote para o engine, com a chave de cada linha lida da página. Linha marcada para apagar
    /// não entra como Update. `None` sem chave ou com chave NULL na página (não deveria acontecer).
    pub fn change_set(&self, key_columns: &[String]) -> Option<ChangeSet> {
        if key_columns.is_empty() {
            return None;
        }
        let rows = self.rows.as_ref()?;
        let key_indexes: Vec<usize> = key_columns
            .iter()
            .map(|k| rows.columns.iter().position(|c| c == k))
            .collect::<Option<_>>()?;
        let key_of = |row: usize| -> Option<Vec<String>> {
            let cells = rows.rows.get(row)?;
            key_indexes
                .iter()
                .map(|i| cells.get(*i).cloned().flatten())
                .collect()
        };
        let mut updates = Vec::new();
        for (row, map) in &self.pending {
            if self.deleted.contains(row) {
                continue;
            }
            let set: Vec<(String, Option<String>)> = rows
                .columns
                .iter()
                .filter_map(|c| map.get(c).map(|v| (c.clone(), v.clone())))
                .collect();
            updates.push(RowUpdate {
                key: key_of(*row)?,
                set,
            });
        }
        let deletes = self
            .deleted
            .iter()
            .map(|row| key_of(*row))
            .collect::<Option<Vec<_>>>()?;
        Some(ChangeSet {
            key_columns: key_columns.to_vec(),
            updates,
            deletes,
        })
    }
}

/// A paleta ⌘K (tela 3.2): busca entre os bancos da conexão.
#[derive(Debug, Default)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
    /// Pede o foco do campo uma vez, ao abrir; depois o usuário manda.
    pub focus_pending: bool,
}

/// Por que o lote não pôde ser montado: sem chave, ou uma linha da página sem valor de chave.
fn batch_error(keys: &[String]) -> &'static str {
    if keys.is_empty() {
        "a tabela não tem chave primária: só leitura"
    } else {
        "não foi possível montar o lote: linha sem chave na página; descarte e recarregue"
    }
}

pub struct App<D: Driver> {
    pub(crate) deps: Deps<D>,
    pub(crate) view: View,
    pub(crate) status: HashMap<ConnectionId, Status>,
    pub(crate) workspaces: HashMap<ConnectionId, Workspace>,
    pub(crate) palette: Option<Palette>,
    /// Painel direito visível (botão na barra superior; abre sozinho na primeira seleção).
    pub(crate) side_panel_open: bool,
    pub(crate) panel_mode: PanelMode,
    pub(crate) connections: Connections,
    pub(crate) form: Option<ConnectionForm>,
    /// Último erro de arquivo ou chaveiro, mostrado no rodapé até o próximo sucesso.
    pub(crate) notice: Option<String>,
}

impl<D> App<D>
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    /// Cria o app e aplica o tema ao contexto do egui.
    pub fn new(cc: &eframe::CreationContext<'_>, deps: Deps<D>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self::with_deps(deps)
    }

    /// Sem o `CreationContext`: para testes e para o `egui_kittest`.
    pub fn with_deps(deps: Deps<D>) -> Self {
        Self {
            deps,
            view: View::Connections,
            status: HashMap::new(),
            workspaces: HashMap::new(),
            palette: None,
            side_panel_open: false,
            panel_mode: PanelMode::default(),
            connections: Connections::default(),
            form: None,
            notice: None,
        }
    }

    /// Lê o que o engine mandou desde o último frame.
    pub(crate) fn drain_events(&mut self) {
        while let Ok(event) = self.deps.events.try_recv() {
            let id = event.connection().clone();
            match event {
                Event::Connecting { .. } => {
                    self.status.insert(id, Status::Connecting);
                }
                Event::Connected {
                    server_version,
                    databases,
                    ..
                } => {
                    self.status.insert(
                        id.clone(),
                        Status::Connected {
                            server_version,
                            databases,
                        },
                    );
                    self.workspaces.insert(id.clone(), Workspace::default());
                    self.view = View::Workspace { connection: id };
                }
                Event::Failed { error, .. } => {
                    self.status.insert(id, Status::Failed(error.to_string()));
                }
                Event::Disconnected { .. } => {
                    // Conexão já removida da lista não volta ao mapa.
                    if self.deps.store.get(&id).is_some() {
                        self.status.insert(id.clone(), Status::Idle);
                    }
                    self.workspaces.remove(&id);
                    if self.view == (View::Workspace { connection: id }) {
                        self.view = View::Connections;
                    }
                }
                Event::DatabaseSelected {
                    database,
                    schemas,
                    current_schema,
                    ..
                } => {
                    let ws = self.workspaces.entry(id.clone()).or_default();
                    *ws = Workspace {
                        database: Some(database.clone()),
                        schemas,
                        current_schema: current_schema.clone(),
                        ..Default::default()
                    };
                    if let Some(schema) = current_schema {
                        ws.loading = true;
                        self.deps.sessions.load_schema(&id, &database, &schema);
                    }
                }
                Event::SchemaLoaded {
                    database,
                    schema,
                    tables,
                    functions,
                    ..
                } => {
                    if let Some(ws) = self
                        .workspaces
                        .get_mut(&id)
                        .filter(|ws| ws.database.as_deref() == Some(database.as_str()))
                    {
                        ws.objects.insert(schema, (tables, functions));
                        ws.loading = false;
                        ws.error = None;
                    }
                }
                Event::ColumnsLoaded {
                    database,
                    schema,
                    table,
                    columns,
                    ..
                } => {
                    if let Some(ws) = self
                        .workspaces
                        .get_mut(&id)
                        .filter(|ws| ws.database.as_deref() == Some(database.as_str()))
                    {
                        ws.columns.insert((schema, table), columns);
                    }
                }
                Event::StatsLoaded {
                    database,
                    schema,
                    table,
                    stats,
                    ..
                } => {
                    if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                        tab.stats = Some(stats);
                    }
                }
                Event::PageLoaded {
                    database,
                    schema,
                    table,
                    page,
                    rows,
                    ..
                } => {
                    // Só a página pedida por último: resposta atrasada de outra ordem ou offset é descartada.
                    if let Some(tab) = self
                        .data_tab_mut(&id, &database, &schema, &table)
                        .filter(|t| t.requested.as_ref() == Some(&page))
                    {
                        tab.accept_page(page, rows);
                    }
                }
                Event::DataFailed {
                    database,
                    schema,
                    table,
                    error,
                    ..
                } => {
                    // Página ou stats: o grid já carregado continua na tela, com o erro em cima. A página pedida

                    // fica: se a falha foi dos stats, a página em voo ainda chega; se foi dela, "Tentar de novo" a repete.
                    if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                        tab.loading = false;
                        tab.error = Some(error.to_string());
                    }
                }
                Event::ChangesApplied {
                    database,
                    schema,
                    table,
                    ..
                } => {
                    // Gravou: fecha a caixa, limpa as pendências e recarrega a página e os stats
                    // (o engine já esvaziou o cache deles).
                    let page = self
                        .data_tab_mut(&id, &database, &schema, &table)
                        .map(|tab| {
                            tab.commit = None;
                            tab.clear_changes();
                            tab.selected_rows.clear();
                            tab.anchor = None;
                            tab.page.clone()
                        });
                    if let Some(page) = page {
                        self.load_data_page(&id, page);
                        self.deps
                            .sessions
                            .load_stats(&id, &database, &schema, &table);
                    }
                }
                Event::ChangesFailed {
                    database,
                    schema,
                    table,
                    error,
                    position,
                    ..
                } => {
                    // Statement ou contagem: desfeito; conexão caída: a mensagem do driver diz. As
                    // pendências ficam para corrigir. O erro vai na caixa, se ela está aberta, senão
                    // na faixa da aba.
                    if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                        match tab.commit.as_mut() {
                            Some(dialog) => {
                                dialog.running = false;
                                dialog.error = Some(error);
                                dialog.failed = position;
                            }
                            None => tab.error = Some(error),
                        }
                    }
                }
                Event::CatalogFailed {
                    database, error, ..
                } => {
                    // Só do banco selecionado, ou da tentativa de abrir o primeiro.
                    if let Some(ws) = self
                        .workspaces
                        .get_mut(&id)
                        .filter(|ws| ws.database.as_deref().is_none_or(|d| d == database))
                    {
                        ws.loading = false;
                        ws.error = Some(if ws.database.is_none() {
                            format!("{database}: {error}")
                        } else {
                            error.to_string()
                        });
                    }
                }
            }
        }
    }

    fn data_tab_mut(
        &mut self,
        id: &str,
        database: &str,
        schema: &str,
        table: &str,
    ) -> Option<&mut DataTab> {
        self.workspaces
            .get_mut(id)
            .filter(|ws| ws.database.as_deref() == Some(database))
            .and_then(|ws| ws.data.as_mut())
            .filter(|tab| tab.schema == schema && tab.table == table)
    }

    /// Abre (ou troca) a tabela da aba Data e pede a primeira página e os tamanhos.
    pub(crate) fn open_table(&mut self, id: &ConnectionId, schema: &str, table: &str) {
        let Some(database) = self.workspaces.get(id).and_then(|ws| ws.database.clone()) else {
            return;
        };
        // Com edição pendente a aba fica travada (a árvore desabilita as outras tabelas, com a dica).
        if self.has_pending_changes(id) {
            return;
        }
        let tab = DataTab::new(schema, table);
        let page = tab.page.clone();
        if let Some(ws) = self.workspaces.get_mut(id) {
            ws.data = Some(tab);
        }
        self.deps
            .sessions
            .load_page(id, &database, schema, table, page);
        self.deps.sessions.load_stats(id, &database, schema, table);
    }

    /// Recarrega a aba Data com a página pedida (ordem, limite, offset).
    pub(crate) fn load_data_page(&mut self, id: &ConnectionId, page: Page) {
        let Some(ws) = self.workspaces.get_mut(id) else {
            return;
        };
        let Some(database) = ws.database.clone() else {
            return;
        };
        let Some(tab) = ws.data.as_mut() else { return };
        if tab.has_changes() {
            return;
        }
        tab.requested = Some(page.clone());
        tab.loading = true;
        tab.error = None;
        tab.selected_rows.clear();
        tab.anchor = None;
        let (schema, table) = (tab.schema.clone(), tab.table.clone());
        self.deps
            .sessions
            .load_page(id, &database, &schema, &table, page);
    }

    /// Repete a página pedida (ou a atual) e, se os tamanhos ainda não vieram, os stats.
    pub(crate) fn retry_data(&mut self, id: &ConnectionId) {
        let Some(ws) = self.workspaces.get(id) else {
            return;
        };
        let Some(database) = ws.database.clone() else {
            return;
        };
        let Some(tab) = ws.data.as_ref() else { return };
        let (schema, table, stats_missing) =
            (tab.schema.clone(), tab.table.clone(), tab.stats.is_none());
        let page = tab.requested.clone().unwrap_or_else(|| tab.page.clone());
        self.load_data_page(id, page);
        if stats_missing {
            self.deps
                .sessions
                .load_stats(id, &database, &schema, &table);
        }
    }

    /// Clique numa linha do grid: simples seleciona só ela; ⌘ (Ctrl fora do macOS) alterna; ⇧ estende
    /// da última clicada. Só a primeira seleção (de nenhuma linha para alguma) abre o painel: depois, o
    /// botão "Painel" manda, e o clique numa linha não o reabre.
    pub(crate) fn select_row(
        &mut self,
        id: &ConnectionId,
        index: usize,
        modifiers: egui::Modifiers,
    ) {
        let Some(tab) = self.workspaces.get_mut(id).and_then(|ws| ws.data.as_mut()) else {
            return;
        };
        let was_empty = tab.selected_rows.is_empty();
        if modifiers.shift {
            // Sem âncora (primeiro clique já com ⇧), a linha clicada vira a âncora dos próximos.
            let from = *tab.anchor.get_or_insert(index);
            let (lo, hi) = if from <= index {
                (from, index)
            } else {
                (index, from)
            };
            tab.selected_rows.extend(lo..=hi);
        } else if modifiers.command {
            if !tab.selected_rows.remove(&index) {
                tab.selected_rows.insert(index);
            }
            tab.anchor = Some(index);
        } else {
            let only_this = tab.selected_rows.len() == 1 && tab.selected_rows.contains(&index);
            tab.selected_rows.clear();
            if !only_this {
                tab.selected_rows.insert(index);
            }
            tab.anchor = Some(index);
        }
        if was_empty && !tab.selected_rows.is_empty() {
            self.side_panel_open = true;
        }
    }

    pub(crate) fn has_pending_changes(&self, id: &ConnectionId) -> bool {
        self.workspaces
            .get(id)
            .and_then(|ws| ws.data.as_ref())
            .is_some_and(DataTab::has_changes)
    }

    /// As colunas da chave primária da tabela aberta, na ordem da tabela; vazio sem cache ou sem
    /// chave (tabela só leitura).
    pub(crate) fn key_columns(&self, id: &ConnectionId) -> Vec<String> {
        let Some(ws) = self.workspaces.get(id) else {
            return Vec::new();
        };
        let Some(tab) = ws.data.as_ref() else {
            return Vec::new();
        };
        let Some(columns) = ws.columns.get(&(tab.schema.clone(), tab.table.clone())) else {
            return Vec::new();
        };
        let mut keys: Vec<&Column> = columns.iter().filter(|c| c.is_primary_key).collect();
        keys.sort_by_key(|c| c.position);
        keys.into_iter().map(|c| c.name.clone()).collect()
    }

    /// A tabela aberta aceita edição: tem chave primária (colunas já em cache), página na tela e
    /// nenhuma página em voo (a pendência é por índice da página; uma página nova a invalidaria).
    pub(crate) fn editable(&self, id: &ConnectionId) -> bool {
        let settled = self
            .workspaces
            .get(id)
            .and_then(|ws| ws.data.as_ref())
            .is_some_and(|tab| tab.rows.is_some() && !tab.loading && tab.requested.is_none());
        settled && !self.key_columns(id).is_empty()
    }

    fn data_tab(&mut self, id: &ConnectionId) -> Option<&mut DataTab> {
        self.workspaces.get_mut(id).and_then(|ws| ws.data.as_mut())
    }

    /// Duplo clique numa célula: abre a edição com o valor atual (pendente ou da página).
    pub(crate) fn start_edit(&mut self, id: &ConnectionId, row: usize, col: usize) {
        if !self.editable(id) {
            return;
        }
        let Some(tab) = self.data_tab(id) else { return };
        if tab.deleted.contains(&row) {
            return;
        }
        let Some(value) = tab.cell_value(row, col) else {
            return;
        };
        let original = value.map(str::to_owned);
        tab.editing = Some(CellEdit {
            row,
            col,
            text: original.clone().unwrap_or_default(),
            original,
            focus_pending: true,
        });
    }

    /// Fim da edição da célula: Enter (ou perder o foco) confirma como pendente, Esc descarta. Abrir
    /// uma célula NULL e sair sem digitar não é mudança: texto vazio sobre NULL continua NULL.
    pub(crate) fn finish_edit(&mut self, id: &ConnectionId, confirm: bool) {
        let Some(tab) = self.data_tab(id) else { return };
        let Some(edit) = tab.editing.take() else {
            return;
        };
        if confirm {
            let value = if edit.text.is_empty() && edit.original.is_none() {
                None
            } else {
                Some(edit.text)
            };
            tab.set_cell(edit.row, edit.col, value);
        }
    }

    /// Campo do Form: o valor vale para todas as linhas selecionadas (o lote do Update).
    pub(crate) fn set_field(&mut self, id: &ConnectionId, column: &str, value: Option<String>) {
        if !self.editable(id) {
            return;
        }
        let Some(tab) = self.data_tab(id) else { return };
        let Some(col) = tab
            .rows
            .as_ref()
            .and_then(|r| r.columns.iter().position(|c| c == column))
        else {
            return;
        };
        for row in tab.selected_rows.clone() {
            if !tab.deleted.contains(&row) {
                tab.set_cell(row, col, value.clone());
            }
        }
    }

    pub(crate) fn revert_field(&mut self, id: &ConnectionId, column: &str) {
        let Some(tab) = self.data_tab(id) else { return };
        for row in tab.selected_rows.clone() {
            tab.revert_cell(row, column);
        }
    }

    /// ⌫ ou o botão Delete: marca as linhas selecionadas para apagar; todas já marcadas, desmarca.
    pub(crate) fn toggle_delete(&mut self, id: &ConnectionId) {
        if !self.editable(id) {
            return;
        }
        let Some(tab) = self.data_tab(id) else { return };
        if tab.selected_rows.is_empty() {
            return;
        }
        let all_marked = tab.selected_rows.iter().all(|r| tab.deleted.contains(r));
        for row in tab.selected_rows.clone() {
            if all_marked {
                tab.deleted.remove(&row);
            } else {
                tab.deleted.insert(row);
            }
        }
        tab.editing = None;
    }

    /// Commit (botão ou ⌘S): confirma a célula em edição, monta o lote e abre a caixa com o SQL
    /// exato; o usuário executa.
    pub(crate) fn open_commit(&mut self, id: &ConnectionId) {
        self.finish_edit(id, true);
        let keys = self.key_columns(id);
        let Some(tab) = self.data_tab(id) else { return };
        if !tab.has_changes() || tab.commit.is_some() {
            return;
        }
        let (updates, deletes) = tab.change_counts();
        let (schema, table) = (tab.schema.clone(), tab.table.clone());
        let Some(changes) = tab.change_set(&keys) else {
            tab.error = Some(batch_error(&keys).to_owned());
            return;
        };
        match preview_sql(&schema, &table, &changes) {
            Ok(sql) => {
                tab.commit = Some(CommitDialog {
                    sql,
                    updates,
                    deletes,
                    running: false,
                    error: None,
                    failed: None,
                });
            }
            Err(error) => tab.error = Some(error.to_string()),
        }
    }

    /// Executar na caixa: manda o lote ao engine; o resultado chega por evento.
    pub(crate) fn execute_changes(&mut self, id: &ConnectionId) {
        let keys = self.key_columns(id);
        let Some(ws) = self.workspaces.get_mut(id) else {
            return;
        };
        let Some(database) = ws.database.clone() else {
            return;
        };
        let Some(tab) = ws.data.as_mut() else { return };
        let Some(running) = tab.commit.as_ref().map(|d| d.running) else {
            return;
        };
        if running {
            return;
        }
        let changes = tab.change_set(&keys);
        let Some(dialog) = tab.commit.as_mut() else {
            return;
        };
        let Some(changes) = changes else {
            dialog.error = Some(batch_error(&keys).to_owned());
            return;
        };
        dialog.running = true;
        dialog.error = None;
        dialog.failed = None;
        let (schema, table) = (tab.schema.clone(), tab.table.clone());
        self.deps
            .sessions
            .apply_changes(id, &database, &schema, &table, &changes);
    }

    pub(crate) fn cancel_commit(&mut self, id: &ConnectionId) {
        if let Some(tab) = self.data_tab(id)
            && !tab.commit.as_ref().is_some_and(|d| d.running)
        {
            tab.commit = None;
        }
    }

    /// Descartar: limpa as pendências sem tocar no banco.
    pub(crate) fn discard_changes(&mut self, id: &ConnectionId) {
        if let Some(tab) = self.data_tab(id)
            && tab.commit.is_none()
        {
            tab.clear_changes();
        }
    }

    pub(crate) fn status_of(&self, id: &str) -> Status {
        self.status.get(id).cloned().unwrap_or(Status::Idle)
    }

    /// O conteúdo da janela, separado do `eframe::App` para ser testável. O `ui` raiz do eframe
    /// vem sem margem nem fundo; cada tela abre seus painéis.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.drain_events();
        match self.view.clone() {
            View::Connections => crate::connections::show(self, ui),
            View::Workspace { connection } => crate::workspace::show(self, ui, &connection),
        }
    }
}

impl<D> eframe::App for App<D>
where
    D: Driver + 'static,
    D::Conn: 'static,
{
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui(ui);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::{Duration, Instant};

    use hy_dbplus_engine::{Connection, ConnectionConfig, MemorySecrets, Secret, Tag};

    use super::*;
    use crate::connections::ConnectionForm;

    pub(crate) fn deps<D: Driver + 'static>(driver: D, rt: &tokio::runtime::Runtime) -> Deps<D>
    where
        D::Conn: 'static,
    {
        let path = std::env::temp_dir()
            .join(format!("hy-dbplus-ui-{}", uuid_like()))
            .join("connections.toml");
        let store = ConnectionStore::load(path).unwrap();
        let (sessions, events) = Sessions::new(driver, rt.handle().clone(), || {});
        Deps {
            store,
            secrets: Box::new(MemorySecrets::default()),
            sessions,
            events,
        }
    }

    fn uuid_like() -> String {
        Connection::new("x", Tag::Local, ConnectionConfig::default()).id
    }

    /// Cadeia inteira sem renderizar: formulário → store + senha → conectar → evento → tela 3.2.
    /// Precisa do PostgreSQL local (`HY_DBPLUS_TEST_PG`: host, porta, usuário, banco e, se houver, a senha).
    #[test]
    fn criar_conectar_e_abrir_a_conexao() {
        let Some(url) = std::env::var_os("HY_DBPLUS_TEST_PG") else {
            return;
        };
        let pg = tokio_postgres_config(url);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::with_deps(deps(PostgresDriver::default(), &rt));
        let mut form = ConnectionForm::new();
        form.name = "pg-localhost".into();
        form.host = pg.0;
        form.port = pg.1.to_string();
        form.user = pg.2;
        form.database = pg.3;
        form.password = pg.4;
        let c = form.build().unwrap();
        app.deps.store.upsert(c.clone());
        app.deps.store.save().unwrap();
        if !form.password.is_empty() {
            app.deps
                .secrets
                .set(&c.id, &Secret::new(form.password.clone()))
                .unwrap();
        }
        let text = std::fs::read_to_string(app.deps.store.path()).unwrap();
        assert!(
            !text.contains(&form.password) || form.password.is_empty(),
            "senha no TOML"
        );
        crate::connections::connect(&mut app, &c);
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            app.drain_events();
            if matches!(
                app.status_of(&c.id),
                Status::Connected { .. } | Status::Failed(_)
            ) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let status = app.status_of(&c.id);
        assert!(
            matches!(&status, Status::Connected { databases, .. } if !databases.is_empty()),
            "{status:?}"
        );
        assert_eq!(
            app.view,
            View::Workspace {
                connection: c.id.clone()
            }
        );
        app.deps.sessions.disconnect(&c.id);
        app.drain_events();
        assert_eq!(app.view, View::Connections);
        assert_eq!(app.status_of(&c.id), Status::Idle);
        let _ = std::fs::remove_dir_all(app.deps.store.path().parent().unwrap());
    }

    /// (host, porta, usuário, banco, senha) de uma URL do libpq, sem depender do tokio-postgres aqui.
    fn tokio_postgres_config(url: std::ffi::OsString) -> (String, u16, String, String, String) {
        let url = url.to_string_lossy().into_owned();
        let rest = url
            .trim_start_matches("postgres://")
            .trim_start_matches("postgresql://");
        let (auth, hostdb) = rest.split_once('@').unwrap_or(("postgres", rest));
        let (user, password) = auth.split_once(':').unwrap_or((auth, ""));
        let (hostport, db) = hostdb.split_once('/').unwrap_or((hostdb, "postgres"));
        let (host, port) = hostport.split_once(':').unwrap_or((hostport, "5432"));
        (
            host.into(),
            port.parse().unwrap_or(5432),
            user.into(),
            db.into(),
            password.into(),
        )
    }
}
