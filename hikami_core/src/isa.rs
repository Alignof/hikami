//! ISA extensions implemented by the host harts.

use fdt::Fdt;

/// Return whether the hart `hart_id` implements the extension `name` (e.g. `"sstc"`).
///
/// It reads `riscv,isa-extensions` of the cpu node in `device_tree`.
/// For device trees without it, it falls back to `riscv,isa`,
/// where only multi-letter extensions can be found.
pub fn has_extension(device_tree: &Fdt, hart_id: usize, name: &str) -> bool {
    let Some(cpu) = device_tree.cpus().find(|cpu| cpu.ids().first() == hart_id) else {
        return false;
    };

    if let Some(extensions) = cpu.property("riscv,isa-extensions") {
        return extensions
            .value
            .split(|&c| c == b'\0')
            .any(|ext| ext == name.as_bytes());
    }

    cpu.property("riscv,isa")
        .and_then(|isa| isa.as_str())
        .is_some_and(|isa| isa.split('_').skip(1).any(|ext| ext == name))
}
