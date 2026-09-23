use phx_core::KernelMap;

fn main() {
    let map: KernelMap<u64, u32> = KernelMap::new();
    for _entry in &map {}
}
