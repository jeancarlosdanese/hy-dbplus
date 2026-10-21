//! hy-dbplus: o app desktop. Cria a janela (eframe) e entrega o resto à `ui-egui`.
//! No M0 a janela entra no passo seguinte; por ora o binário só existe e sai com 0.

fn main() {
    println!(
        "hy-dbplus {} (M0: janela no próximo passo)",
        env!("CARGO_PKG_VERSION")
    );
}
