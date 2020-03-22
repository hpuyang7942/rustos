use core::panic::PanicInfo;

use crate::console::kprintln;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kprintln!("            (");
    kprintln!("       (      )     )");
    kprintln!("         )   (    (");
    kprintln!("        (          `");
    kprintln!("    .-^^^^^^^^^^^^^^^^^-.");
    kprintln!("  (~~~~~~~~~~~~~~~~~~~~~~)");
    kprintln!("    ~^^^^^^^^^^^^^^^^^^~");
    kprintln!("     `================`");
    kprintln!("");
    kprintln!("    The pi is overdoe.");
    kprintln!("");
    kprintln!("---------- PANIC ----------");
    match _info.location() {
        Some(m) => {
            kprintln!("FILE: {}", m.file());
            kprintln!("LINE: {}", m.line());
            kprintln!("COLUMN: {}", m.column());
        },
        None => {kprintln!("NONE");}
    }  
    kprintln!();
    match _info.message() {
        Some(m) => {
            kprintln!("Message");
        }
        None => {kprintln!("NO MESSAGE");}
    }
    loop{ unsafe {asm!("wfe")} }
}