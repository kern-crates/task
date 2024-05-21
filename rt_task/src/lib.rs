#![no_std]

#[macro_use]
extern crate axlog2;
extern crate alloc;
use alloc::sync::Arc;

use core::panic::PanicInfo;
use axtype::{align_up_4k, align_down_4k, phys_to_virt, virt_to_phys};
use driver_block::{ramdisk, BlockDriverOps};
use axdriver::{prelude::*, AxDeviceContainer};
use axhal::mem::memory_regions;
use fstree::FsStruct;

/// Entry
#[no_mangle]
pub extern "Rust" fn runtime_main(cpu_id: usize, _dtb_pa: usize) {
    assert_eq!(cpu_id, 0);

    axlog2::init();
    axlog2::set_max_level("debug");
    info!("[rt_task]: ... cpuid {}", cpu_id);

    axhal::cpu::init_primary(cpu_id);

    let start = align_up_4k(virt_to_phys(_ekernel as usize));
    let end = align_down_4k(axconfig::PHYS_MEMORY_END);
    axalloc::global_init(phys_to_virt(start), end - start);

    info!("Initialize kernel page table...");
    remap_kernel_memory().expect("remap kernel memoy failed");

    let ctx = Arc::new(taskctx::init_sched_info());
    unsafe {
        let ptr = Arc::into_raw(ctx.clone());
        axhal::cpu::set_current_task_ptr(ptr);
    }

    task::init();
    run_queue::init();

    info!("[rt_task]: ok!");
    axhal::misc::terminate();
}

fn remap_kernel_memory() -> Result<(), axhal::paging::PagingError> {
    use axhal::paging::PageTable;
    use axhal::paging::{reuse_page_table_root, setup_page_table_root};
    use axhal::mem::phys_to_virt;

    let mut kernel_page_table = PageTable::try_new()?;
    for r in memory_regions() {
        kernel_page_table.map_region(
            phys_to_virt(r.paddr),
            r.paddr,
            r.size,
            r.flags.into(),
            true,
        )?;
    }
    setup_page_table_root(kernel_page_table);

    Ok(())
}

pub fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    arch_boot::panic(info)
}

extern "C" {
    fn _ekernel();
}
