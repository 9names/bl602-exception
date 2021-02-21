use bl602_hal as hal;
use hal::prelude::_embedded_hal_blocking_delay_DelayUs;
use hal::pac;

#[export_name = "ExceptionHandler"]
extern "C" fn exception_handler() {
    // If we've hit an exception, the reason should be in mcause
    // Technically, we should mask the interrupt bit, but this
    // function won't be called if that's set
    let cause = riscv::register::mcause::read().bits();
    // List of possible exceptions (from RISC-V priv spec v1.10 3.1.20 table 3.6)
    // Exceptions > 7 don't make sense for this hardware, so not listing them
    const EXCEPTION_STRINGS: &[&str] = &[
        "Instruction address misaligned", // 0
        "Instruction access fault",       // 1
        "Illegal instruction",            // 2
        "Breakpoint",                     // 3
        "Load address misaligned",        // 4
        "Load access fault",              // 5
        "Store/AMO address misaligned",   // 6
        "Store/AMO access fault",         // 7
    ];
    
    let dp = unsafe { pac::Peripherals::steal() };
    // Print header for exception report
    for c in "Exception:\r\n\t".as_bytes() {
        while dp.UART.uart_fifo_config_1.read().tx_fifo_cnt().bits() < 1 {}
        dp.UART
            .uart_fifo_wdata
            .write(|w| unsafe { w.bits(*c as u32) });
    }

    // Print the exception string associated with mcause value, if mcause is valid
    if cause < EXCEPTION_STRINGS.len() {
        for c in EXCEPTION_STRINGS[cause].as_bytes() {
            while dp.UART.uart_fifo_config_1.read().tx_fifo_cnt().bits() < 1 {}
            dp.UART
                .uart_fifo_wdata
                .write(|w| unsafe { w.bits(*c as u32) });
        }
    }

    // Let the user know we're going to try to reset
    for c in "\r\n Resetting system now... \r\n".as_bytes() {
        while dp.UART.uart_fifo_config_1.read().tx_fifo_cnt().bits() < 1 {}
        dp.UART
            .uart_fifo_wdata
            .write(|w| unsafe { w.bits(*c as u32) });
    }

    // Wait until we've finished transmitting the last of our error messages
    // before doing anything else
    while dp.UART.uart_status.read().sts_utx_bus_busy().bit_is_set() {}
    let glb = unsafe { &*pac::GLB::ptr() };

    // Clear the system reset bits, they're edge triggered and won't work otherwise
    glb.swrst_cfg2.modify(|_r, w| {
        w.reg_ctrl_cpu_reset()
            .clear_bit()
            .reg_ctrl_sys_reset()
            .clear_bit()
            .reg_ctrl_pwron_rst()
            .clear_bit()
    });
    // Assert cpu + sys reset bits. Don't assert power-on reset, that would be a lie
    glb.swrst_cfg2.modify(|_r, w| {
        w.reg_ctrl_cpu_reset()
            .set_bit()
            .reg_ctrl_sys_reset()
            .set_bit()
    });
    // We need to do nothing until the system has reset, so delay a bit
    let mut d = bl602_hal::delay::McycleDelay::new(160_000_000);
    d.try_delay_us(10).unwrap()
}