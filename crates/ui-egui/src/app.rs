//! O app egui: estado, roteamento entre as telas e leitura dos eventos do engine a cada frame.
//! Genérico no driver para o teste de interação rodar com um engine falso; o binário usa
//! [`PgApp`].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use std::time::{Duration, Instant};

use hy_dbplus_engine::{
    ChangeSet, Column, ConnectionId, ConnectionStore, DatabaseInfo, Driver, Event, Function,
    Outcome, Page, PostgresDriver, QueryStore, QueryTabId, RowUpdate, Schema, SecretStore,
    Sessions, Table, TableStats, TableStructure, Template, TextRows, preview_sql, table_ddl,
    template_sql,
};

use crate::connections::{ConnectionForm, Connections};
use crate::theme;

/// O app com o driver PostgreSQL, como o binário o cria.
pub type PgApp = App<PostgresDriver>;

/// O que o app recebe pronto do binário: o engine é dele, a UI só usa.
pub struct Deps<D: Driver> {
    pub store: ConnectionStore,
    pub secrets: Box<dyn SecretStore>,
    /// As queries das abas SQL em arquivo (FEAT-011).
    pub queries: QueryStore,
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

/// Qual tela está na frente. O workspace mostra o banco ativo (`App::active`) ou, sem banco
/// aberto, a conexão em foco (`App::focus`) com o convite ao ⌘K.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Connections,
    Workspace,
}

/// Um banco aberto: a conexão e o nome do banco. É a chave de `App::workspaces` e o item da
/// faixa de bancos (FEAT-008).
pub type DbKey = (ConnectionId, String);

/// O que a UI sabe de uma conexão fora de qualquer banco: o banco que o ⌘K está abrindo e o
/// erro da última tentativa (tela 3.2, "Nenhum banco selecionado").
#[derive(Debug, Default, Clone)]
pub struct ConnState {
    pub opening: Option<String>,
    pub error: Option<String>,
}

/// O que o usuário pediu para fechar com edição pendente: a confirmação decide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseTarget {
    Database(DbKey),
    Tab(DbKey, usize),
    Connection(ConnectionId),
}

/// Uma aba do banco: de tabela (Data/Structure/Index) ou de SQL (FEAT-010).
#[derive(Debug, Clone)]
pub enum Tab {
    /// Em `Box` as duas: a enum fica pequena (clippy `large_enum_variant`).
    Table(Box<DataTab>),
    Query(Box<QueryTab>),
}

impl Tab {
    pub fn table(&self) -> Option<&DataTab> {
        match self {
            Tab::Table(t) => Some(t.as_ref()),
            Tab::Query(_) => None,
        }
    }

    pub fn table_mut(&mut self) -> Option<&mut DataTab> {
        match self {
            Tab::Table(t) => Some(t.as_mut()),
            Tab::Query(_) => None,
        }
    }

    pub fn query(&self) -> Option<&QueryTab> {
        match self {
            Tab::Query(q) => Some(q.as_ref()),
            Tab::Table(_) => None,
        }
    }

    pub fn query_mut(&mut self) -> Option<&mut QueryTab> {
        match self {
            Tab::Query(q) => Some(q.as_mut()),
            Tab::Table(_) => None,
        }
    }

    /// O rótulo da aba: `tabela` (ou `schema.tabela` fora do schema atual) ou o título da SQL.
    pub fn title(&self, current_schema: Option<&str>) -> String {
        match self {
            Tab::Table(t) if current_schema == Some(t.schema.as_str()) => t.table.clone(),
            Tab::Table(t) => format!("{}.{}", t.schema, t.table),
            Tab::Query(q) => q.title.clone(),
        }
    }

    /// Fechar perde algo: pendência de edição, ou texto no editor.
    pub fn would_lose_work(&self) -> bool {
        match self {
            Tab::Table(t) => t.has_changes() || t.editing.is_some(),
            Tab::Query(q) => !q.buffer.trim().is_empty(),
        }
    }

    /// Algo roda no banco por esta aba: Commit em execução ou SQL em execução.
    pub fn busy(&self) -> bool {
        match self {
            Tab::Table(t) => t.commit_running(),
            Tab::Query(q) => q.running,
        }
    }
}

/// O resultado de um statement do editor, como chegou do engine.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub index: usize,
    pub sql: String,
    pub result: Result<Outcome, String>,
    pub elapsed: Duration,
    /// Linha do grid mostrada em JSON no painel direito (⌘-clique).
    pub selected_row: Option<usize>,
}

/// Uma aba "SQL Query" (FEAT-010): o buffer, o limite, a execução em curso e os resultados.
#[derive(Debug, Clone)]
pub struct QueryTab {
    pub id: QueryTabId,
    pub title: String,
    pub buffer: String,
    /// Cursor do editor, em caracteres (o egui conta assim); vira byte para `statement_at`.
    pub cursor: usize,
    pub limit: Option<usize>,
    pub running: bool,
    pub started: Option<Instant>,
    pub results: Vec<QueryResult>,
    pub active_result: usize,
    pub notice: Option<String>,
    /// O texto como está no arquivo (FEAT-011): só grava de novo quando o buffer mudou.
    pub saved: String,
    /// O texto cuja gravação falhou: o varrimento por frame não tenta de novo até ele mudar (sem
    /// isso, uma falha persistente refaria a escrita e reescreveria o aviso a cada repaint).
    pub save_failed: Option<String>,
    /// Cursor a aplicar no `TextEdit` no próximo frame (FEAT-013: o SQL gerado entra no fim do
    /// buffer e o cursor vai para ele); o editor consome, grava no estado do widget e pede o foco.
    pub pending_cursor: Option<usize>,
}

pub const DEFAULT_LIMIT: Option<usize> = Some(100);

/// A aba SQL que recebe o SQL gerado (FEAT-013): a ativa, se é SQL, senão a última SQL do banco.
fn query_target(ws: &Workspace) -> Option<usize> {
    ws.active_tab
        .filter(|&i| ws.tabs.get(i).is_some_and(|t| t.query().is_some()))
        .or_else(|| ws.tabs.iter().rposition(|t| t.query().is_some()))
}

/// A tabela ou view do catálogo já carregado na árvore.
fn find_table(ws: &Workspace, schema: &str, table: &str) -> Option<Table> {
    ws.objects
        .get(schema)
        .and_then(|(tables, _)| tables.iter().find(|t| t.name == table))
        .cloned()
}

impl QueryTab {
    pub fn new(id: QueryTabId) -> Self {
        Self {
            id,
            title: format!("SQL Query {id}"),
            buffer: String::new(),
            cursor: 0,
            limit: DEFAULT_LIMIT,
            running: false,
            started: None,
            results: Vec::new(),
            active_result: 0,
            notice: None,
            saved: String::new(),
            save_failed: None,
            pending_cursor: None,
        }
    }

    /// Uma aba que volta do arquivo ao abrir o banco: o texto, sem resultados.
    pub fn restored(id: QueryTabId, sql: String) -> Self {
        Self {
            saved: sql.clone(),
            buffer: sql,
            ..Self::new(id)
        }
    }

    /// O cursor em bytes, para `statement_at`.
    pub fn cursor_byte(&self) -> usize {
        self.buffer
            .char_indices()
            .nth(self.cursor)
            .map_or(self.buffer.len(), |(b, _)| b)
    }
}

/// Um item do menu de contexto da tabela (FEAT-013): um modelo do catálogo ou o `CREATE` (DDL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlItem {
    Template(Template),
    Create,
}

/// SQL gerado pelo menu da árvore esperando o catálogo: as colunas (`ColumnsLoaded`) ou a
/// estrutura (`StructureLoaded`) da tabela. Um por banco; pedido novo substitui o anterior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSql {
    pub schema: String,
    pub table: String,
    pub item: SqlItem,
}

/// Um banco aberto (conexão + banco) e o catálogo que a UI já recebeu (tela 3.3).
#[derive(Debug, Default)]
pub struct Workspace {
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
    /// As abas abertas neste banco (FEAT-008: tabela; FEAT-010: SQL) e a ativa.
    pub tabs: Vec<Tab>,
    pub active_tab: Option<usize>,
    /// Um `BEGIN` do editor sem `COMMIT`/`ROLLBACK`: a transação é da conexão do banco, não da
    /// aba; todas as abas SQL avisam e o Commit do grid espera.
    pub open_transaction: bool,
    /// O último número de aba SQL deste banco (`SQL Query n`, arquivo `n.sql`): a numeração é por
    /// banco, para a aba e o arquivo casarem (FEAT-011).
    pub next_query_id: QueryTabId,
    /// Um erro dentro da transação aberta (qualquer um, inclusive cancel): o servidor a marca como
    /// abortada e o `COMMIT` vai desfazer tudo. Volta a `false` quando a transação acaba.
    pub transaction_aborted: bool,
    /// SQL do menu da árvore (FEAT-013) esperando as colunas ou a estrutura chegarem.
    pub pending_sql: Option<PendingSql>,
}

impl Workspace {
    /// A aba ativa, se é de tabela.
    pub fn data(&self) -> Option<&DataTab> {
        self.tabs.get(self.active_tab?)?.table()
    }

    pub fn data_mut(&mut self) -> Option<&mut DataTab> {
        self.tabs.get_mut(self.active_tab?)?.table_mut()
    }

    /// A aba ativa, se é de SQL.
    pub fn query(&self) -> Option<&QueryTab> {
        self.tabs.get(self.active_tab?)?.query()
    }

    pub fn query_mut(&mut self) -> Option<&mut QueryTab> {
        self.tabs.get_mut(self.active_tab?)?.query_mut()
    }

    pub fn tab_index(&self, schema: &str, table: &str) -> Option<usize> {
        self.tabs.iter().position(|t| {
            t.table()
                .is_some_and(|t| t.schema == schema && t.table == table)
        })
    }

    /// A aba daquela tabela, ativa ou não (os eventos de dados chegam por tabela).
    pub fn tab_mut(&mut self, schema: &str, table: &str) -> Option<&mut DataTab> {
        let i = self.tab_index(schema, table)?;
        self.tabs.get_mut(i)?.table_mut()
    }

    /// A aba de SQL com aquele id, ativa ou não (os eventos do editor chegam por aba).
    pub fn query_tab_mut(&mut self, id: QueryTabId) -> Option<&mut QueryTab> {
        self.tabs
            .iter_mut()
            .filter_map(Tab::query_mut)
            .find(|q| q.id == id)
    }

    /// Alguma aba de tabela com edição pendente (↻ e desconectar precisam saber).
    pub fn has_pending_changes(&self) -> bool {
        self.tabs
            .iter()
            .filter_map(Tab::table)
            .any(DataTab::has_changes)
    }

    /// Alguma aba com Commit ou SQL em execução: nada fecha até o resultado chegar.
    pub fn busy(&self) -> bool {
        self.tabs.iter().any(Tab::busy)
    }

    /// Fechar o banco perderia algo: pendência de edição ou célula aberta numa aba de tabela. O texto
    /// das abas SQL não conta: é salvo em arquivo antes de o banco fechar (FEAT-011).
    pub fn would_lose_work(&self) -> bool {
        self.tabs
            .iter()
            .any(|t| t.table().is_some() && t.would_lose_work())
    }

    /// Fecha uma aba. Era a ativa: a da direita assume (a da esquerda se era a última); estava à
    /// esquerda da ativa: a ativa só muda de índice.
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        self.active_tab = match self.active_tab {
            _ if self.tabs.is_empty() => None,
            Some(a) if a > index => Some(a - 1),
            Some(a) if a == index => Some(index.min(self.tabs.len() - 1)),
            other => other,
        };
    }

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

/// A sub-aba de uma aba de tabela (FEAT-009): Data é o padrão; Structure e Index são só leitura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabView {
    #[default]
    Data,
    Structure,
    Index,
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
    /// Sub-aba na tela; a estrutura (Structure, Index, DDL) chega uma vez e fica (o ↻ a esvazia).
    pub view: TabView,
    pub structure: Option<Arc<TableStructure>>,
    pub structure_requested: bool,
    /// Erro da última carga da estrutura (separado de `error`, que é da página e dos stats).
    pub structure_error: Option<String>,
    pub ddl_open: bool,
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
            view: TabView::default(),
            structure: None,
            structure_requested: false,
            structure_error: None,
            ddl_open: false,
        }
    }

    /// Há edição ou exclusão pendente: ordenar, paginar e recarregar esta aba ficam travados até o
    /// Commit ou o Descartar (e o ↻ do banco, que esvaziaria as colunas dela).
    pub fn has_changes(&self) -> bool {
        !self.pending.is_empty() || !self.deleted.is_empty()
    }

    pub fn commit_running(&self) -> bool {
        self.commit.as_ref().is_some_and(|d| d.running)
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

/// Depois de um statement bem-sucedido: `BEGIN`/`START TRANSACTION` abre, `COMMIT`/`ROLLBACK`/`END`
/// fecha (`ROLLBACK TO` não: volta ao savepoint com a transação aberta); o resto não muda (o
/// editor avisa "transação aberta" na barra).
fn transaction_state(sql: &str, open: bool) -> bool {
    match transaction_effect(sql) {
        TxEffect::Opens => true,
        TxEffect::Commits { chain } | TxEffect::RollsBack { chain } => chain,
        TxEffect::Recovers | TxEffect::Keeps => open,
    }
}

/// O que um statement bem-sucedido faz com a transação da conexão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxEffect {
    /// `BEGIN`, `START TRANSACTION`.
    Opens,
    /// `COMMIT`/`END`; com `AND CHAIN` abre outra na hora (`AND NO CHAIN` é o `COMMIT` comum).
    Commits {
        chain: bool,
    },
    /// `ROLLBACK`/`ABORT`; com `AND CHAIN` abre outra na hora.
    RollsBack {
        chain: bool,
    },
    /// `ROLLBACK [WORK | TRANSACTION] TO [SAVEPOINT] x`: a transação segue e sai do estado abortado.
    Recovers,
    Keeps,
}

fn transaction_effect(sql: &str) -> TxEffect {
    let head: String = sql
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    let words: Vec<&str> = head.split_whitespace().collect();
    let chain = |rest: &[&str]| rest.windows(2).any(|w| w == ["AND", "CHAIN"]);
    match words.as_slice() {
        ["BEGIN", ..] | ["START", "TRANSACTION", ..] => TxEffect::Opens,
        ["ROLLBACK", rest @ ..] if rest.contains(&"TO") => TxEffect::Recovers,
        ["COMMIT", rest @ ..] | ["END", rest @ ..] => TxEffect::Commits { chain: chain(rest) },
        ["ROLLBACK", rest @ ..] | ["ABORT", rest @ ..] => {
            TxEffect::RollsBack { chain: chain(rest) }
        }
        _ => TxEffect::Keeps,
    }
}

/// `COMMIT`/`END` (com ou sem `AND CHAIN`) de uma transação abortada: o servidor faz ROLLBACK e
/// responde OK; nada foi gravado. `ROLLBACK`/`ABORT` de transação abortada não é perda.
fn undoes_commit(effect: TxEffect, aborted: bool) -> bool {
    aborted && matches!(effect, TxEffect::Commits { .. })
}

/// Erro do Commit do grid quando o editor SQL do banco está executando.
const SQL_RUNNING_ERROR: &str =
    "há uma execução em curso no editor SQL deste banco: aguarde o resultado antes do Commit";

pub struct App<D: Driver> {
    pub(crate) deps: Deps<D>,
    pub(crate) view: View,
    pub(crate) status: HashMap<ConnectionId, Status>,
    /// Os bancos abertos, por (conexão, banco).
    pub(crate) workspaces: HashMap<DbKey, Workspace>,
    /// A ordem dos bancos abertos na faixa e o selecionado nela.
    pub(crate) rail: Vec<DbKey>,
    pub(crate) active: Option<DbKey>,
    /// A conexão do workspace na tela (a do banco ativo, ou a recém-conectada sem banco).
    pub(crate) focus: Option<ConnectionId>,
    pub(crate) conn_state: HashMap<ConnectionId, ConnState>,
    /// O que está esperando confirmação para fechar com edição pendente.
    pub(crate) confirm_close: Option<CloseTarget>,
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
            rail: Vec::new(),
            active: None,
            focus: None,
            conn_state: HashMap::new(),
            confirm_close: None,
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
            self.apply_event(event);
        }
    }

    /// Aplica um evento do engine ao estado (separado para os testes injetarem eventos).
    pub(crate) fn apply_event(&mut self, event: Event) {
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
                self.conn_state.insert(id.clone(), ConnState::default());
                self.focus = Some(id);
                self.view = View::Workspace;
            }
            Event::Failed { error, .. } => {
                self.status.insert(id, Status::Failed(error.to_string()));
            }
            Event::Disconnected { .. } => {
                // Conexão já removida da lista não volta ao mapa.
                if self.deps.store.get(&id).is_some() {
                    self.status.insert(id.clone(), Status::Idle);
                }
                self.forget_connection(&id);
            }
            Event::DatabaseSelected {
                database,
                schemas,
                current_schema,
                ..
            } => {
                // Banco aberto (ou já estava: o engine responde do cache): entra na faixa e vira
                // o ativo. Já aberto na UI, só vira o ativo, sem perder árvore e aba.
                let key = (id.clone(), database.clone());
                if let Some(state) = self.conn_state.get_mut(&id) {
                    if state.opening.as_deref() == Some(database.as_str()) {
                        state.opening = None;
                    }
                    state.error = None;
                }
                if !self.workspaces.contains_key(&key) {
                    let mut ws = Workspace {
                        schemas,
                        current_schema: current_schema.clone(),
                        ..Default::default()
                    };
                    if let Some(schema) = current_schema {
                        ws.loading = true;
                        self.deps.sessions.load_schema(&id, &database, &schema);
                    }
                    // As abas SQL salvas voltam com o texto e o mesmo número, sem resultados; a
                    // primeira fica ativa (ainda não há aba de tabela).
                    match self.deps.queries.list(&id, &database) {
                        Ok(saved) => {
                            for q in saved {
                                ws.next_query_id = ws.next_query_id.max(q.id);
                                ws.tabs
                                    .push(Tab::Query(Box::new(QueryTab::restored(q.id, q.sql))));
                            }
                            if !ws.tabs.is_empty() {
                                ws.active_tab = Some(0);
                            }
                        }
                        Err(e) => {
                            if let Some(state) = self.conn_state.get_mut(&id) {
                                state.error = Some(format!("queries salvas: {e}"));
                            }
                        }
                    }
                    self.workspaces.insert(key.clone(), ws);
                    self.rail.push(key.clone());
                }
                self.focus = Some(id);
                self.active = Some(key);
            }
            Event::SchemaLoaded {
                database,
                schema,
                tables,
                functions,
                ..
            } => {
                if let Some(ws) = self.workspaces.get_mut(&(id, database)) {
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
                let key = (id, database);
                if let Some(ws) = self.workspaces.get_mut(&key) {
                    ws.columns.insert((schema.clone(), table.clone()), columns);
                }
                self.complete_pending_sql(&key, &schema, &table, None);
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
            Event::StructureLoaded {
                database,
                schema,
                table,
                structure,
                ..
            } => {
                let structure = Arc::new(structure);
                if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                    tab.structure = Some(Arc::clone(&structure));
                    tab.structure_requested = false;
                    tab.structure_error = None;
                }
                self.complete_pending_sql(&(id, database), &schema, &table, Some(&structure));
            }
            Event::StructureFailed {
                database,
                schema,
                table,
                error,
                ..
            } => {
                // Só as sub-abas Structure/Index: a página e os stats não mudam, e o "Tentar de
                // novo" delas pede a estrutura de novo.
                if let Some(tab) = self.data_tab_mut(&id, &database, &schema, &table) {
                    tab.structure_requested = false;
                    tab.structure_error = Some(error.to_string());
                }
                self.discard_pending_sql(&(id, database), &schema, &table, &error.to_string());
            }
            Event::SqlStarted { database, tab, .. } => {
                if let Some(q) = self.query_tab_mut(&id, &database, tab) {
                    q.running = true;
                    q.started = Some(Instant::now());
                    q.results.clear();
                    q.active_result = 0;
                    q.notice = None;
                }
            }
            Event::SqlResult {
                database,
                tab,
                index,
                sql,
                result,
                elapsed,
                ..
            } => {
                // Só a aba que está rodando: um resultado em voo de um banco fechado e
                // reaberto (ou de uma aba que já não existe) não mexe em nada.
                if !self
                    .query_tab_mut(&id, &database, tab)
                    .is_some_and(|q| q.running)
                {
                    return;
                }
                // O COMMIT de uma transação abortada vira ROLLBACK no servidor, com "OK" na resposta.
                let mut commit_undone = false;
                if let Some(ws) = self.workspaces.get_mut(&(id.clone(), database.clone())) {
                    if result.is_ok() {
                        let effect = transaction_effect(&sql);
                        ws.open_transaction = transaction_state(&sql, ws.open_transaction);
                        if effect != TxEffect::Keeps {
                            // Abrir, fechar ou voltar a um savepoint sai do estado abortado.
                            commit_undone = undoes_commit(effect, ws.transaction_aborted);
                            ws.transaction_aborted = false;
                        }
                    } else if ws.open_transaction {
                        // Qualquer ERROR dentro da transação a aborta no servidor.
                        ws.transaction_aborted = true;
                    }
                }
                if let Some(q) = self.query_tab_mut(&id, &database, tab) {
                    q.results.push(QueryResult {
                        index,
                        sql,
                        result: result.map_err(|e| e.to_string()),
                        elapsed,
                        selected_row: None,
                    });
                    q.active_result = q.results.len() - 1;
                    if commit_undone {
                        q.notice = Some(
                            "A transação estava abortada: o COMMIT virou ROLLBACK, nada foi gravado"
                                .into(),
                        );
                    }
                }
            }
            Event::SqlFinished {
                database,
                tab,
                cancelled,
                ..
            } => {
                if let Some(q) = self.query_tab_mut(&id, &database, tab) {
                    q.running = false;
                    q.started = None;
                    if cancelled {
                        q.notice = Some("Execução cancelada".into());
                    }
                }
            }
            Event::SqlBusy { database, tab, .. } => {
                if let Some(q) = self.query_tab_mut(&id, &database, tab) {
                    q.notice =
                        Some("Outra execução ainda roda neste banco; aguarde ou cancele".into());
                }
            }
            Event::DatabaseClosed { database, .. } => {
                self.forget_database(&(id, database));
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
                    self.load_tab_page(&(id.clone(), database.clone()), &schema, &table, page);
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
                // Do banco que o ⌘K estava abrindo (vai ao painel da conexão) ou de um banco
                // aberto (schema que não carregou: vai à árvore dele).
                if let Some(ws) = self.workspaces.get_mut(&(id.clone(), database.clone())) {
                    ws.loading = false;
                    ws.error = Some(error.to_string());
                    // O SQL do menu que esperava as colunas não vem mais; o erro já está na árvore.
                    ws.pending_sql = None;
                } else if let Some(state) = self.conn_state.get_mut(&id) {
                    // Dois ⌘K seguidos: o erro do primeiro ainda aparece, mesmo que `opening` já
                    // seja o segundo.
                    if state.opening.as_deref() == Some(database.as_str()) {
                        state.opening = None;
                    }
                    state.error = Some(format!("{database}: {error}"));
                }
            }
        }
    }

    fn query_tab_mut(
        &mut self,
        id: &str,
        database: &str,
        tab: QueryTabId,
    ) -> Option<&mut QueryTab> {
        self.workspaces
            .get_mut(&(id.to_owned(), database.to_owned()))
            .and_then(|ws| ws.query_tab_mut(tab))
    }

    fn data_tab_mut(
        &mut self,
        id: &str,
        database: &str,
        schema: &str,
        table: &str,
    ) -> Option<&mut DataTab> {
        self.workspaces
            .get_mut(&(id.to_owned(), database.to_owned()))
            .and_then(|ws| ws.tab_mut(schema, table))
    }

    /// Tira um banco da faixa e do mapa; se era o ativo, o último da faixa assume.
    pub(crate) fn forget_database(&mut self, key: &DbKey) {
        // Fechado pelo engine (conexão caída inclusive): o texto das abas SQL não se perde. Conexão
        // que saiu da lista não: o diretório dela acabou de ser apagado e voltaria como lixo órfão.
        if self.deps.store.get(&key.0).is_some()
            && let Err(e) = self.save_workspace_queries(key)
        {
            // A aba (e o aviso dela) some com o workspace: o erro vai para onde ainda há tela.
            if let Some(state) = self.conn_state.get_mut(&key.0) {
                state.error = Some(e.clone());
            }
            self.notice = Some(e);
        }
        self.workspaces.remove(key);
        self.rail.retain(|k| k != key);
        if matches!(&self.confirm_close, Some(CloseTarget::Database(k) | CloseTarget::Tab(k, _)) if k == key)
        {
            self.confirm_close = None;
        }
        if self.active.as_ref() == Some(key) {
            self.active = self.rail.last().cloned();
            if let Some(active) = &self.active {
                self.focus = Some(active.0.clone());
            }
        }
    }

    /// Conexão fechada: todos os bancos dela saem; sem banco em foco, volta à lista.
    fn forget_connection(&mut self, id: &ConnectionId) {
        for key in self.workspace_keys_of(id) {
            self.forget_database(&key);
        }
        self.conn_state.remove(id);
        if matches!(&self.confirm_close, Some(CloseTarget::Connection(c)) if c == id) {
            self.confirm_close = None;
        }
        if self.focus.as_ref() == Some(id) {
            self.focus = self.active.as_ref().map(|k| k.0.clone());
            if self.focus.is_none() {
                self.view = View::Connections;
            }
        }
    }

    /// O workspace de uma conexão: o ativo, se é dela, senão o primeiro dela na faixa.
    pub(crate) fn key_of(&self, id: &str) -> Option<DbKey> {
        self.active
            .iter()
            .chain(self.rail.iter())
            .find(|k| k.0 == id)
            .cloned()
    }

    #[cfg(test)]
    pub(crate) fn workspace_of(&self, id: &str) -> Option<&Workspace> {
        self.workspaces.get(&self.key_of(id)?)
    }

    /// Seleciona um banco na faixa.
    pub(crate) fn activate(&mut self, key: &DbKey) {
        if self.workspaces.contains_key(key) {
            self.focus = Some(key.0.clone());
            self.active = Some(key.clone());
            self.view = View::Workspace;
        }
    }

    /// ⌘K: abre o banco (ou só o seleciona, se já está na faixa).
    pub(crate) fn open_database(&mut self, id: &ConnectionId, database: &str) {
        let key = (id.clone(), database.to_owned());
        if self.workspaces.contains_key(&key) {
            self.activate(&key);
            return;
        }
        let state = self.conn_state.entry(id.clone()).or_default();
        state.opening = Some(database.to_owned());
        state.error = None;
        self.deps.sessions.select_database(id, database);
    }

    /// × na faixa: fecha o banco no engine; `DatabaseClosed` o tira da faixa.
    /// Fecha o banco depois de salvar as abas SQL (falha ao salvar vira aviso na aba e o banco fecha
    /// mesmo assim: quem chama já descartou ou já tratou o erro).
    pub(crate) fn close_database(&mut self, key: &DbKey) {
        let _ = self.save_workspace_queries(key);
        self.deps.sessions.close_database(&key.0, &key.1);
    }

    /// Grava o texto da aba SQL `index` se mudou desde o último salvamento (texto vazio apaga o
    /// arquivo). Falha vai ao aviso da aba e não trava nada.
    pub(crate) fn save_query_tab(&mut self, key: &DbKey, index: usize) -> Result<(), String> {
        let Some(q) = self
            .workspaces
            .get_mut(key)
            .and_then(|ws| ws.tabs.get_mut(index))
            .and_then(Tab::query_mut)
        else {
            return Ok(());
        };
        if q.buffer == q.saved {
            return Ok(());
        }
        match self.deps.queries.save(&key.0, &key.1, q.id, &q.buffer) {
            Ok(()) => {
                q.saved = q.buffer.clone();
                q.save_failed = None;
                Ok(())
            }
            Err(e) => {
                let message = format!("não foi possível salvar a query {}: {e}", q.title);
                q.notice = Some(message.clone());
                q.save_failed = Some(q.buffer.clone());
                Err(message)
            }
        }
    }

    /// A cada frame: toda aba SQL cujo texto mudou e cujo editor não tem o foco é salva. Cobre
    /// perder o foco por Esc ou clique, trocar de aba, trocar de banco na faixa e voltar à lista de
    /// conexões (o `TextEdit` de uma aba que não está na tela não é desenhado, então ninguém leria
    /// o `lost_focus` dele). O custo por frame é a comparação do texto; a escrita só sai quando mudou.
    pub(crate) fn autosave_unfocused_queries(&mut self, ctx: &egui::Context) {
        let pending: Vec<(DbKey, usize)> = self
            .workspaces
            .iter()
            .flat_map(|(key, ws)| {
                ws.tabs.iter().enumerate().filter_map(move |(index, tab)| {
                    let q = tab.query()?;
                    let dirty = q.buffer != q.saved && q.save_failed.as_ref() != Some(&q.buffer);
                    if !dirty {
                        return None;
                    }
                    let focused = ctx.memory(|m| m.has_focus(crate::editor::editor_id(key, q.id)));
                    (!focused).then(|| (key.clone(), index))
                })
            })
            .collect();
        for (key, index) in pending {
            let _ = self.save_query_tab(&key, index);
        }
    }

    /// Todas as abas SQL do banco: antes de fechar o banco, desconectar ou sair. Devolve o primeiro
    /// erro (a aba dele ficou com o aviso).
    fn save_workspace_queries(&mut self, key: &DbKey) -> Result<(), String> {
        let count = self.workspaces.get(key).map_or(0, |ws| ws.tabs.len());
        let mut first_error = None;
        for index in 0..count {
            if let Err(e) = self.save_query_tab(key, index) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Todas as abas SQL de todos os bancos abertos (sair do app).
    pub(crate) fn save_all_queries(&mut self) -> Result<(), String> {
        let keys: Vec<DbKey> = self.workspaces.keys().cloned().collect();
        let mut first_error = None;
        for key in &keys {
            if let Err(e) = self.save_workspace_queries(key) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// "Descartar e fechar" de uma aba SQL: apaga o arquivo dela e esvazia o buffer, para o
    /// fechamento não salvar de novo. Falha ao apagar deixa `saved` como estava: o arquivo ficou.
    fn discard_query_tab(&mut self, key: &DbKey, index: usize) {
        let Some(q) = self
            .workspaces
            .get_mut(key)
            .and_then(|ws| ws.tabs.get_mut(index))
            .and_then(Tab::query_mut)
        else {
            return;
        };
        q.buffer.clear();
        match self.deps.queries.remove(&key.0, &key.1, q.id) {
            Ok(()) => q.saved.clear(),
            Err(e) => {
                if let Some(state) = self.conn_state.get_mut(&key.0) {
                    state.error = Some(format!("não foi possível apagar a query salva: {e}"));
                }
            }
        }
    }

    fn workspace_keys_of(&self, id: &ConnectionId) -> Vec<DbKey> {
        self.rail.iter().filter(|k| &k.0 == id).cloned().collect()
    }

    /// Abre a tabela numa aba nova (ou só foca a aba dela, se já está aberta) e pede a primeira
    /// página e os tamanhos.
    pub(crate) fn open_table(&mut self, key: &DbKey, schema: &str, table: &str) {
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        if let Some(i) = ws.tab_index(schema, table) {
            ws.active_tab = Some(i);
            return;
        }
        let tab = DataTab::new(schema, table);
        let page = tab.page.clone();
        ws.tabs.push(Tab::Table(Box::new(tab)));
        ws.active_tab = Some(ws.tabs.len() - 1);
        let (id, database) = key;
        self.deps
            .sessions
            .load_page(id, database, schema, table, page);
        self.deps.sessions.load_stats(id, database, schema, table);
    }

    /// Recarrega a aba Data com a página pedida (ordem, limite, offset).
    pub(crate) fn load_data_page(&mut self, key: &DbKey, page: Page) {
        let Some((schema, table)) = self
            .workspaces
            .get(key)
            .and_then(|ws| ws.data())
            .map(|t| (t.schema.clone(), t.table.clone()))
        else {
            return;
        };
        self.load_tab_page(key, &schema, &table, page);
    }

    /// Pede uma página para a aba daquela tabela, ativa ou não (o `ChangesApplied` recarrega a aba
    /// do evento, que pode já não ser a ativa).
    fn load_tab_page(&mut self, key: &DbKey, schema: &str, table: &str, page: Page) {
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        let (id, database) = key;
        let Some(tab) = ws.tab_mut(schema, table) else {
            return;
        };
        if tab.has_changes() {
            return;
        }
        tab.requested = Some(page.clone());
        tab.loading = true;
        tab.error = None;
        tab.selected_rows.clear();
        tab.anchor = None;
        self.deps
            .sessions
            .load_page(id, database, schema, table, page);
    }

    /// Repete a página pedida (ou a atual) e, se os tamanhos ainda não vieram, os stats.
    pub(crate) fn retry_data(&mut self, key: &DbKey) {
        let Some(ws) = self.workspaces.get(key) else {
            return;
        };
        let Some(tab) = ws.data() else { return };
        let (schema, table, stats_missing) =
            (tab.schema.clone(), tab.table.clone(), tab.stats.is_none());
        let page = tab.requested.clone().unwrap_or_else(|| tab.page.clone());
        self.load_data_page(key, page);
        if stats_missing {
            self.deps
                .sessions
                .load_stats(&key.0, &key.1, &schema, &table);
        }
    }

    /// Clique numa linha do grid: simples seleciona só ela; ⌘ (Ctrl fora do macOS) alterna; ⇧ estende
    /// da última clicada. Só a primeira seleção (de nenhuma linha para alguma) abre o painel: depois, o
    /// botão "Painel" manda, e o clique numa linha não o reabre.
    pub(crate) fn select_row(&mut self, key: &DbKey, index: usize, modifiers: egui::Modifiers) {
        let Some(tab) = self.workspaces.get_mut(key).and_then(|ws| ws.data_mut()) else {
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

    /// Commit ou SQL em execução em alguma aba do banco: nada fecha até o resultado chegar
    /// (fechar esconderia um lote que ainda grava, ou o erro dele).
    pub(crate) fn busy(&self, key: &DbKey) -> bool {
        self.workspaces.get(key).is_some_and(Workspace::busy)
    }

    pub(crate) fn connection_busy(&self, id: &str) -> bool {
        self.workspaces.iter().any(|(k, ws)| k.0 == id && ws.busy())
    }

    /// **+ SQL** (⌘T): uma aba "SQL Query N" nova no banco, ativa.
    pub(crate) fn open_query_tab(&mut self, key: &DbKey) {
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        ws.next_query_id += 1;
        ws.tabs
            .push(Tab::Query(Box::new(QueryTab::new(ws.next_query_id))));
        ws.active_tab = Some(ws.tabs.len() - 1);
    }

    /// Menu de contexto da tabela (FEAT-013): monta o modelo e o põe na aba SQL, ou pede ao engine
    /// o que falta (colunas ou estrutura) e completa quando o evento chegar. Nada roda.
    pub(crate) fn generate_sql(&mut self, key: &DbKey, schema: &str, table: &str, item: SqlItem) {
        enum Action {
            Insert(String),
            Columns,
            Structure,
        }
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        let Some(t) = find_table(ws, schema, table) else {
            return;
        };
        let action = match item {
            SqlItem::Template(template) if !template.needs_columns() => {
                Action::Insert(template_sql(template, &t, &[]))
            }
            SqlItem::Template(template) => {
                match ws.columns.get(&(schema.to_owned(), table.to_owned())) {
                    Some(columns) => Action::Insert(template_sql(template, &t, columns)),
                    None => Action::Columns,
                }
            }
            SqlItem::Create => {
                // A estrutura já cacheada na aba Data da tabela serve na hora.
                let cached = ws
                    .tabs
                    .iter()
                    .filter_map(Tab::table)
                    .find(|d| d.schema == schema && d.table == table)
                    .and_then(|d| d.structure.clone());
                match cached {
                    Some(structure) => Action::Insert(table_ddl(&structure)),
                    None => Action::Structure,
                }
            }
        };
        let pending = PendingSql {
            schema: schema.to_owned(),
            table: table.to_owned(),
            item,
        };
        match action {
            Action::Insert(sql) => self.insert_sql(key, &sql),
            Action::Columns => {
                ws.pending_sql = Some(pending);
                self.deps
                    .sessions
                    .load_columns(&key.0, &key.1, schema, table);
            }
            Action::Structure => {
                ws.pending_sql = Some(pending);
                self.deps
                    .sessions
                    .load_structure(&key.0, &key.1, schema, table);
            }
        }
    }

    /// `ColumnsLoaded` ou `StructureLoaded` da tabela que o pedido pendente espera: monta e insere.
    /// Evento de outra tabela, ou do tipo que o pedido não espera, não mexe nele.
    fn complete_pending_sql(
        &mut self,
        key: &DbKey,
        schema: &str,
        table: &str,
        structure: Option<&TableStructure>,
    ) {
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        let Some(pending) = ws
            .pending_sql
            .as_ref()
            .filter(|p| p.schema == schema && p.table == table)
        else {
            return;
        };
        let sql = match (pending.item, structure) {
            (SqlItem::Create, Some(structure)) => table_ddl(structure),
            (SqlItem::Template(template), None) => {
                let Some(columns) = ws.columns.get(&(schema.to_owned(), table.to_owned())) else {
                    return;
                };
                let Some(t) = find_table(ws, schema, table) else {
                    ws.pending_sql = None;
                    return;
                };
                template_sql(template, &t, columns)
            }
            _ => return,
        };
        ws.pending_sql = None;
        self.insert_sql(key, &sql);
    }

    /// Falha do catálogo para a tabela do pedido pendente: descarta e avisa na aba SQL que receberia
    /// o texto (ou na faixa da conexão, se o banco não tem aba SQL): é onde o usuário está olhando.
    fn discard_pending_sql(&mut self, key: &DbKey, schema: &str, table: &str, error: &str) {
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        if !ws
            .pending_sql
            .as_ref()
            .is_some_and(|p| p.schema == schema && p.table == table)
        {
            return;
        }
        ws.pending_sql = None;
        let message = format!("{schema}.{table}: {error}");
        match query_target(ws).and_then(|i| ws.tabs.get_mut(i).and_then(Tab::query_mut)) {
            Some(q) => q.notice = Some(message),
            None => {
                if let Some(state) = self.conn_state.get_mut(&key.0) {
                    state.error = Some(message);
                }
            }
        }
    }

    /// Põe o SQL no fim da aba SQL ativa (ou da última do banco, ou de uma nova), separado por uma
    /// linha em branco, com o cursor no começo dele: Run Current roda exatamente ele.
    fn insert_sql(&mut self, key: &DbKey, sql: &str) {
        if self.workspaces.get(key).and_then(query_target).is_none() {
            self.open_query_tab(key);
        }
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        let Some(index) = query_target(ws) else {
            return;
        };
        ws.active_tab = Some(index);
        let Some(q) = ws.tabs.get_mut(index).and_then(Tab::query_mut) else {
            return;
        };
        let mut buffer = q.buffer.trim_end().to_owned();
        if !buffer.is_empty() {
            buffer.push_str("\n\n");
        }
        let start = buffer.chars().count();
        buffer.push_str(sql);
        buffer.push('\n');
        q.buffer = buffer;
        q.cursor = start;
        q.pending_cursor = Some(start);
    }

    /// Run Current (⌘↩): o statement sob o cursor.
    pub(crate) fn run_current(&mut self, key: &DbKey) {
        let Some(q) = self.workspaces.get(key).and_then(|ws| ws.query()) else {
            return;
        };
        let Some(statement) = hy_dbplus_sql::statement_at(&q.buffer, q.cursor_byte()) else {
            self.run(key, Vec::new());
            return;
        };
        if !statement.complete && statement.sql.contains(';') {
            // A cauda incompleta vai até o fim do buffer e pode levar statements inteiros que o
            // usuário não pediu (o tokenizador erra em sintaxe que o servidor aceita): não roda.
            if let Some(q) = self.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
                q.notice = Some(
                    "Statement incompleto (string, $$ ou comentário sem fechar) com `;` depois: o \
                     editor não sabe onde ele termina. Corrija, tire o `;` ou use Run All"
                        .into(),
                );
            }
            return;
        }
        let incomplete = !statement.complete;
        self.run(key, vec![statement.sql]);
        if incomplete && let Some(q) = self.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
            q.notice = Some("Statement incompleto (string, $$ ou comentário sem fechar)".into());
        }
    }

    /// Run All (⌘⇧↩): todos os statements do buffer, em ordem; o engine para no primeiro erro.
    pub(crate) fn run_all(&mut self, key: &DbKey) {
        let Some(q) = self.workspaces.get(key).and_then(|ws| ws.query()) else {
            return;
        };
        let statements = hy_dbplus_sql::split(&q.buffer)
            .into_iter()
            .map(|s| s.sql)
            .collect();
        self.run(key, statements);
    }

    fn run(&mut self, key: &DbKey, statements: Vec<String>) {
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        // Commit do grid em curso neste banco: o editor esperaria o lock da conexão e um Cancelar
        // dele derrubaria o lote no servidor (o cancel é por conexão, não por statement).
        let commit_running = ws
            .tabs
            .iter()
            .any(|t| matches!(t, Tab::Table(t) if t.commit_running()));
        let Some(q) = ws.query_mut() else {
            return;
        };
        if commit_running {
            q.notice = Some("Há um Commit em curso neste banco; aguarde o resultado".into());
            return;
        }
        if q.running {
            q.notice = Some("Já há uma execução em curso; aguarde ou cancele".into());
            return;
        }
        if statements.is_empty() {
            q.notice = Some("Nada para executar".into());
            return;
        }
        let (id, limit) = (q.id, q.limit);
        self.deps
            .sessions
            .run_sql(&key.0, &key.1, id, statements, limit);
    }

    pub(crate) fn cancel_sql(&mut self, key: &DbKey) {
        if self
            .workspaces
            .get(key)
            .and_then(|ws| ws.query())
            .is_some_and(|q| q.running)
        {
            self.deps.sessions.cancel_sql(&key.0, &key.1);
        }
    }

    /// Beautify (⌘I): o buffer inteiro; `sql::beautify` devolve a entrada intacta se não puder.
    pub(crate) fn beautify(&mut self, key: &DbKey) {
        if let Some(q) = self.workspaces.get_mut(key).and_then(|ws| ws.query_mut()) {
            let formatted = hy_dbplus_sql::beautify(&q.buffer);
            if formatted != q.buffer {
                q.buffer = formatted;
                q.cursor = q.cursor.min(q.buffer.chars().count());
            }
        }
    }

    /// Troca a sub-aba da aba ativa; Structure e Index pedem a estrutura ao engine uma vez (ou de
    /// novo depois de uma falha, pelo "Tentar de novo").
    pub(crate) fn set_view(&mut self, key: &DbKey, view: TabView) {
        let Some(tab) = self.data_tab(key) else {
            return;
        };
        tab.view = view;
        if view == TabView::Data || tab.structure.is_some() || tab.structure_requested {
            return;
        }
        tab.structure_requested = true;
        tab.structure_error = None;
        let (schema, table) = (tab.schema.clone(), tab.table.clone());
        self.deps
            .sessions
            .load_structure(&key.0, &key.1, &schema, &table);
    }

    /// Alguma aba do banco com edição pendente.
    pub(crate) fn has_pending_changes(&self, key: &DbKey) -> bool {
        self.workspaces
            .get(key)
            .is_some_and(Workspace::has_pending_changes)
    }

    /// × numa aba ou ⌘W: fecha na hora, ou pede confirmação se há pendência.
    pub(crate) fn request_close_tab(&mut self, key: &DbKey, index: usize) {
        let Some(tab) = self.workspaces.get(key).and_then(|ws| ws.tabs.get(index)) else {
            return;
        };
        if tab.busy() {
            return;
        }
        // Pendência de edição, célula aberta com texto ou buffer de SQL: trabalho a perder.
        if tab.would_lose_work() {
            self.confirm_close = Some(CloseTarget::Tab(key.clone(), index));
        } else {
            // Aba SQL vazia: o arquivo (se havia) some junto. Falha ao apagar não pode ficar no aviso
            // da aba, que fecha agora: vai para a conexão e para o aviso do app.
            if let Err(e) = self.save_query_tab(key, index) {
                if let Some(state) = self.conn_state.get_mut(&key.0) {
                    state.error = Some(e.clone());
                }
                self.notice = Some(e);
            }
            if let Some(ws) = self.workspaces.get_mut(key) {
                ws.close_tab(index);
            }
        }
    }

    /// × na faixa: fecha na hora (salvando as abas SQL), ou pede confirmação se alguma aba de
    /// tabela tem pendência.
    pub(crate) fn request_close_database(&mut self, key: &DbKey) {
        if self.busy(key) {
            return;
        }
        if self
            .workspaces
            .get(key)
            .is_some_and(Workspace::would_lose_work)
        {
            self.confirm_close = Some(CloseTarget::Database(key.clone()));
        } else if let Err(e) = self.save_workspace_queries(key) {
            // Não fecha sem o texto salvo: a aba fica na tela com o aviso.
            if let Some(state) = self.conn_state.get_mut(&key.0) {
                state.error = Some(format!("o banco continua aberto: {e}"));
            }
        } else {
            self.deps.sessions.close_database(&key.0, &key.1);
        }
    }

    /// "Desconectar": fecha na hora, ou pede confirmação se alguma aba da conexão tem pendência.
    pub(crate) fn request_disconnect(&mut self, id: &ConnectionId) {
        if self.connection_busy(id) {
            return;
        }
        let loses = self
            .workspaces
            .iter()
            .any(|(k, ws)| &k.0 == id && ws.would_lose_work());
        if loses {
            self.confirm_close = Some(CloseTarget::Connection(id.clone()));
            return;
        }
        let mut first_error = None;
        for key in self.workspace_keys_of(id) {
            if let Err(e) = self.save_workspace_queries(&key) {
                first_error.get_or_insert(e);
            }
        }
        match first_error {
            // Não desconecta sem o texto salvo: a aba fica na tela com o aviso.
            Some(e) => {
                if let Some(state) = self.conn_state.get_mut(id) {
                    state.error = Some(format!("a conexão continua aberta: {e}"));
                }
            }
            None => self.deps.sessions.disconnect(id),
        }
    }

    /// "Descartar e fechar" na confirmação. Descartar uma aba SQL apaga o arquivo dela; descartar o
    /// banco ou a conexão (a caixa só aparece por pendência em aba de tabela) descarta só a edição:
    /// as queries salvas ficam, como no fechamento sem pendência.
    pub(crate) fn confirm_close_now(&mut self) {
        match self.confirm_close.take() {
            Some(CloseTarget::Database(key)) => self.close_database(&key),
            Some(CloseTarget::Tab(key, index)) => {
                self.discard_query_tab(&key, index);
                if let Some(ws) = self.workspaces.get_mut(&key) {
                    ws.close_tab(index);
                }
            }
            Some(CloseTarget::Connection(id)) => {
                for key in self.workspace_keys_of(&id) {
                    let _ = self.save_workspace_queries(&key);
                }
                self.deps.sessions.disconnect(&id);
            }
            None => {}
        }
    }

    /// As colunas da chave primária da tabela aberta, na ordem da tabela; vazio sem cache ou sem
    /// chave (tabela só leitura).
    pub(crate) fn key_columns(&self, key: &DbKey) -> Vec<String> {
        let Some(ws) = self.workspaces.get(key) else {
            return Vec::new();
        };
        let Some(tab) = ws.data() else {
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
    pub(crate) fn editable(&self, key: &DbKey) -> bool {
        let settled = self
            .workspaces
            .get(key)
            .and_then(|ws| ws.data())
            .is_some_and(|tab| tab.rows.is_some() && !tab.loading && tab.requested.is_none());
        settled && !self.key_columns(key).is_empty()
    }

    fn data_tab(&mut self, key: &DbKey) -> Option<&mut DataTab> {
        self.workspaces.get_mut(key).and_then(|ws| ws.data_mut())
    }

    /// Duplo clique numa célula: abre a edição com o valor atual (pendente ou da página).
    pub(crate) fn start_edit(&mut self, key: &DbKey, row: usize, col: usize) {
        if !self.editable(key) {
            return;
        }
        let Some(tab) = self.data_tab(key) else {
            return;
        };
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
    pub(crate) fn finish_edit(&mut self, key: &DbKey, confirm: bool) {
        let Some(tab) = self.data_tab(key) else {
            return;
        };
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
    pub(crate) fn set_field(&mut self, key: &DbKey, column: &str, value: Option<String>) {
        if !self.editable(key) {
            return;
        }
        let Some(tab) = self.data_tab(key) else {
            return;
        };
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

    pub(crate) fn revert_field(&mut self, key: &DbKey, column: &str) {
        let Some(tab) = self.data_tab(key) else {
            return;
        };
        for row in tab.selected_rows.clone() {
            tab.revert_cell(row, column);
        }
    }

    /// ⌫ ou o botão Delete: marca as linhas selecionadas para apagar; todas já marcadas, desmarca.
    pub(crate) fn toggle_delete(&mut self, key: &DbKey) {
        if !self.editable(key) {
            return;
        }
        let Some(tab) = self.data_tab(key) else {
            return;
        };
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
    pub(crate) fn open_commit(&mut self, key: &DbKey) {
        self.finish_edit(key, true);
        let keys = self.key_columns(key);
        let open_transaction = self
            .workspaces
            .get(key)
            .is_some_and(|ws| ws.open_transaction);
        let sql_running = self.sql_running(key);
        let Some(tab) = self.data_tab(key) else {
            return;
        };
        if !tab.has_changes() || tab.commit.is_some() {
            return;
        }
        if sql_running {
            // Espelho de `run`: o lote esperaria o lock da conexão e um Cancelar do editor o
            // derrubaria no servidor (o cancel é por conexão).
            tab.error = Some(SQL_RUNNING_ERROR.into());
            return;
        }
        if open_transaction {
            // O lote faria BEGIN dentro da transação do editor e o COMMIT dele gravaria tudo junto.
            tab.error = Some(
                "há uma transação aberta no editor SQL deste banco: feche com COMMIT ou ROLLBACK antes"
                    .into(),
            );
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
    /// Alguma aba SQL do banco com execução em curso.
    fn sql_running(&self, key: &DbKey) -> bool {
        self.workspaces.get(key).is_some_and(|ws| {
            ws.tabs
                .iter()
                .any(|t| matches!(t, Tab::Query(q) if q.running))
        })
    }

    pub(crate) fn execute_changes(&mut self, key: &DbKey) {
        if self.sql_running(key) {
            if let Some(tab) = self.data_tab(key) {
                tab.commit = None;
                tab.error = Some(SQL_RUNNING_ERROR.into());
            }
            return;
        }
        let keys = self.key_columns(key);
        let Some(ws) = self.workspaces.get_mut(key) else {
            return;
        };
        let Some(tab) = ws.data_mut() else { return };
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
            .apply_changes(&key.0, &key.1, &schema, &table, &changes);
    }

    pub(crate) fn cancel_commit(&mut self, key: &DbKey) {
        if let Some(tab) = self.data_tab(key)
            && !tab.commit.as_ref().is_some_and(|d| d.running)
        {
            tab.commit = None;
        }
    }

    /// Descartar: limpa as pendências sem tocar no banco.
    pub(crate) fn discard_changes(&mut self, key: &DbKey) {
        if let Some(tab) = self.data_tab(key)
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
        self.autosave_unfocused_queries(ui.ctx());
        match self.view.clone() {
            View::Connections => crate::connections::show(self, ui),
            View::Workspace => crate::workspace::show(self, ui),
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

    /// Sair do app grava o texto de todas as abas SQL abertas (FEAT-011).
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(e) = self.save_all_queries() {
            // Sem tela nesta altura: o stderr é o que resta.
            eprintln!("hy-dbplus: {e}");
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::{Duration, Instant};

    /// O aviso "transação aberta" segue BEGIN/START TRANSACTION e COMMIT/ROLLBACK/END, em qualquer
    /// caixa e com espaço antes; o resto não muda o estado.
    /// Fechar o banco só perde trabalho de aba de tabela (pendência ou célula aberta); texto de aba
    /// SQL é salvo em arquivo e não conta (FEAT-011; 7ª revisão da FEAT-010).
    #[test]
    fn fechar_o_banco_so_perde_trabalho_de_aba_de_tabela() {
        let mut ws = Workspace::default();
        assert!(!ws.would_lose_work());
        let mut q = QueryTab::new(1);
        q.buffer = "select 1".into();
        ws.tabs.push(Tab::Query(Box::new(q)));
        assert!(!ws.would_lose_work(), "texto SQL é salvo, não perdido");
        assert!(
            ws.tabs[0].would_lose_work(),
            "fechar a aba em si ainda pergunta"
        );
        let mut tab = DataTab::new("public", "a");
        tab.editing = Some(CellEdit {
            row: 0,
            col: 0,
            text: "x".into(),
            original: None,
            focus_pending: false,
        });
        ws.tabs.push(Tab::Table(Box::new(tab)));
        assert!(ws.would_lose_work());
    }

    #[test]
    fn transacao_aberta_segue_begin_e_commit() {
        assert!(transaction_state("begin", false));
        assert!(transaction_state(
            "  START TRANSACTION isolation level serializable",
            false
        ));
        assert!(transaction_state("select 1", true));
        assert!(!transaction_state("commit", true));
        assert!(!transaction_state("ROLLBACK;", true));
        // ROLLBACK TO volta ao savepoint: a transação continua (revisão de 22/10/2026).
        assert!(transaction_state("rollback to savepoint a", true));
        assert!(transaction_state("ROLLBACK WORK TO a;", true));
        assert!(!transaction_state("rollback to savepoint a", false));
        // Espaço interno não importa.
        assert!(transaction_state("START  TRANSACTION", false));
        assert!(transaction_state("start\ntransaction;", false));
        // AND CHAIN fecha e abre outra na hora: segue aberta.
        assert!(transaction_state("commit and chain", true));
        assert!(transaction_state("ROLLBACK AND CHAIN;", true));
        assert!(!transaction_state("commit and no chain", true));
        assert_eq!(
            transaction_effect("rollback to savepoint a"),
            TxEffect::Recovers
        );
        assert_eq!(
            transaction_effect("end and chain"),
            TxEffect::Commits { chain: true }
        );
        assert_eq!(
            transaction_effect("abort"),
            TxEffect::RollsBack { chain: false }
        );
        assert_eq!(transaction_effect("select 1"), TxEffect::Keeps);
        // Só COMMIT/END de transação abortada é perda (inclusive AND CHAIN); ROLLBACK não.
        assert!(undoes_commit(TxEffect::Commits { chain: false }, true));
        assert!(undoes_commit(TxEffect::Commits { chain: true }, true));
        assert!(!undoes_commit(TxEffect::Commits { chain: false }, false));
        assert!(!undoes_commit(TxEffect::RollsBack { chain: false }, true));
        assert!(!undoes_commit(TxEffect::Recovers, true));
        assert!(!transaction_state("end", true));
        assert!(!transaction_state("END TRANSACTION", true));
        assert!(!transaction_state("abort", true));
        assert!(!transaction_state("select 1", false));
        assert!(transaction_state("BEGIN; select 1", false));
    }

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
        let queries = QueryStore::new(path.with_file_name("queries"));
        let store = ConnectionStore::load(path).unwrap();
        let (sessions, events) = Sessions::new(driver, rt.handle().clone(), || {});
        Deps {
            store,
            secrets: Box::new(MemorySecrets::default()),
            queries,
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
        assert_eq!(app.view, View::Workspace);
        assert_eq!(app.focus.as_deref(), Some(c.id.as_str()));
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
