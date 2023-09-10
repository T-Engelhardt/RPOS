# Minimal Operating System in Rust for the Raspberry Pi

Fun little project to get video output over HDMI with a minimal monolithic kernel. Only tested on a Raspberry Pi 3.

Based on [rust-raspberrypi-OS-tutorials Tutorial 07](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials/tree/master/07_timestamps) with the global heap from [Tutorial 19](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials/tree/master/19_kernel_heap).

Check [rust-raspberrypi-OS-tutorials](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials) for installations instructions.

## Links

- [RP platform addresses](https://github.com/maccasoft/raspberry-pi/blob/master/kernel/platform.h)

### Mailbox
- [Accessing mailboxes](https://github.com/raspberrypi/firmware/wiki/Accessing-mailboxes)
- [The Property Mailbox Channel](https://jsandler18.github.io/extra/prop-channel.html)

### GPU

- [framebuffer](https://github.com/isometimes/rpi4-osdev/tree/master/part5-framebuffer)
- [rust embedded_graphics_core DrawTarget](https://docs.rs/embedded-graphics-core/latest/embedded_graphics_core/draw_target/trait.DrawTarget.html)