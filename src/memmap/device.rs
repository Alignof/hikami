pub mod initrd;
pub mod plic;
pub mod uart;
pub mod virtio;

use fdt::Fdt;

pub trait Device {
    fn new(device_tree: &Fdt, node_path: &str) -> Self;
    fn size(&self) -> usize;
    fn paddr(&self) -> usize;
    fn vaddr(&self) -> usize;
}
