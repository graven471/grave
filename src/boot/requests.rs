use limine::{
    BaseRevision,
    request::{FramebufferRequest, HhdmRequest, MemmapRequest, RsdpRequest},
};

#[used]
#[unsafe(link_section = ".requests")]
pub(super) static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
pub(super) static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub(super) static HHDM_RESQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub(super) static MEMORY_MAP_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub(super) static RDSP_REQUEST: RsdpRequest = RsdpRequest::new();
