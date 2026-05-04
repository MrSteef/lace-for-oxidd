extern crate cty;
use cty::c_char;
pub use cty::c_int;
use std::ffi::CString;
pub use std::mem::MaybeUninit;

pub struct TreeSearchResult {
    pub maxdepth: usize,
    pub size: usize,
    pub leaves: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TreeType {
    BIN = 0,
    GEO,
    HYBRID,
    BALANCED,
}
type RngState = [u8; 20];
#[repr(C)]
pub struct Node {
    pub _type: TreeType,
    pub height: c_int,
    pub num_children: c_int,
    pub state: RngState,
}

#[link(name = "uts_c", kind = "static")]
extern "C" {
    fn uts_initRoot(root: &mut MaybeUninit<Node>, _type: TreeType);
    fn uts_numChildren(parent: &mut Node) -> c_int;
    fn uts_childType(parent: &Node) -> TreeType;
    fn uts_parseParams(args: c_int, argv: *const *mut c_char);
    pub fn rng_spawn(parent: &mut RngState, child: &mut RngState, spawn_number: c_int);

    pub static mut computeGranularity: c_int;
}
impl Node {
    #[inline(always)]
    pub fn root(_type: TreeType) -> Node {
        let mut nd: MaybeUninit<Node> = MaybeUninit::uninit();
        unsafe {
            uts_initRoot(&mut nd, _type);
            nd.assume_init()
        }
    }
    #[inline(always)]
    pub fn num_children(&mut self) -> c_int {
        unsafe { uts_numChildren(self) }
    }
    #[inline(always)]
    pub fn child_type(&self) -> TreeType {
        unsafe { uts_childType(self) }
    }
}

#[no_mangle]
pub extern "C" fn impl_getName() -> *const c_char {
    "Rust Binding For UTS".as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn impl_parseParam(_key: *const c_char, _val: *const c_char) -> c_int {
    // let (key, val): (&CStr, &CStr) = unsafe { (CStr::from_ptr(key), CStr::from_ptr(val)) };
    // let (key, val) = (key.to_str().unwrap(), val.to_str().unwrap());
    1
}
#[no_mangle]
pub extern "C" fn impl_helpMessage() {
    // describe additional parameters
    println!("   -seq:   run the sequential version");
    println!("   -lace:  run the lace version");
    println!("   -rayon: run the rayon version");
    println!("    if none of the above are passed, all are enabled.");
    println!("   -w <n>: use <n> workers");
}
#[no_mangle]
pub extern "C" fn impl_abort(errcode: c_int) -> ! {
    std::process::exit(errcode);
}

pub fn parse_args(args: Vec<String>) {
    let argv: Vec<CString> = args
        .into_iter()
        // discard cargo arguments, the benchmark arguments
        // use one dash (like "-h", "-t", etc.)
        .filter(|arg| !arg.starts_with("--"))
        .map(|arg| CString::new(arg).unwrap())
        .collect();
    let c_argv: Vec<*mut c_char> = argv.iter().map(|arg| arg.as_ptr() as *mut c_char).collect();
    unsafe { uts_parseParams(c_argv.len() as c_int, c_argv.as_ptr()) };
}
