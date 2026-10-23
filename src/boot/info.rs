#![allow(dead_code)]
use super::requests::{BASE_REVISION, FB_REQUEST, HHDM_RESQUEST, MEMORY_MAP_REQUEST, RDSP_REQUEST};
use crate::memory::map::MemoryMap;

// move to framebuffer.rs when it grows
pub struct FrameBuffer {
    pub addr: *mut u32,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

pub struct BootInfo {
    pub framebuffer: FrameBuffer,
    pub memory_map: MemoryMap,
    pub hhdm_offset: u64,
    pub rsdp_addr: u64,
}

impl BootInfo {
    pub fn new() -> Self {
        assert!(BASE_REVISION.is_supported());

        let fb_resp = FB_REQUEST.response().unwrap();
        let fb = fb_resp.framebuffers()[0];
        let mm_resp = MEMORY_MAP_REQUEST.response().unwrap();
        let hhdm_resp = HHDM_RESQUEST.response().unwrap();
        let rsdp_resp = RDSP_REQUEST.response().unwrap();

        let framebuffer = FrameBuffer {
            addr: fb.address() as *mut u32,
            width: fb.width as usize,
            height: fb.height as usize,
            pitch: fb.pitch as usize,
        };

        let mem_map = MemoryMap {
            entries: mm_resp.entries(),
        };

        return Self {
            framebuffer,
            memory_map: mem_map,
            hhdm_offset: hhdm_resp.offset,
            rsdp_addr: rsdp_resp.address as u64,
        };
    }
}
