use super::*;

struct TextProgram;
impl PlitaProgram for TextProgram {
    fn resize(&mut self, _: Size) -> UiResponse {
        UiResponse::none()
    }
    fn handle_event(&mut self, _: UiEvent) -> UiResponse {
        UiResponse::none()
    }
    fn draw_list(&self) -> DrawList {
        DrawList::new(plita_ui::Color::rgb(0.0, 0.0, 0.0))
    }
}

#[test]
fn text_programs_keep_ime_enabled_by_default() {
    assert!(TextProgram.ime_allowed());
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires a Windows desktop with an installed IME"]
fn native_physical_key_windows_detach_ime_without_affecting_text_windows() {
    use winit::{
        platform::windows::EventLoopBuilderExtWindows,
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };

    #[link(name = "imm32")]
    unsafe extern "system" {
        fn ImmGetContext(hwnd: isize) -> isize;
        fn ImmReleaseContext(hwnd: isize, context: isize) -> i32;
    }

    fn has_ime_context(window: &Window) -> bool {
        let RawWindowHandle::Win32(handle) = window.window_handle().unwrap().as_raw() else {
            panic!("expected a Windows HWND");
        };
        unsafe {
            let context = ImmGetContext(handle.hwnd.get());
            if context != 0 {
                assert_ne!(ImmReleaseContext(handle.hwnd.get(), context), 0);
            }
            context != 0
        }
    }

    struct PhysicalProgram;
    impl PlitaProgram for PhysicalProgram {
        fn ime_allowed(&self) -> bool {
            false
        }
        fn resize(&mut self, _: Size) -> UiResponse {
            UiResponse::none()
        }
        fn handle_event(&mut self, _: UiEvent) -> UiResponse {
            UiResponse::none()
        }
        fn draw_list(&self) -> DrawList {
            TextProgram.draw_list()
        }
    }

    #[derive(Default)]
    struct Probe {
        passed: bool,
    }
    impl ApplicationHandler for Probe {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let attributes = Window::default_attributes()
                .with_title("plita IME context probe")
                .with_visible(false);
            let text = event_loop.create_window(attributes.clone()).unwrap();
            configure_program_ime(&text, &TextProgram);
            assert!(
                has_ime_context(&text),
                "IME must be installed for this native test"
            );
            for _ in 0..3 {
                let emulator = event_loop.create_window(attributes.clone()).unwrap();
                configure_program_ime(&emulator, &TextProgram);
                assert!(has_ime_context(&emulator));
                configure_program_ime(&emulator, &PhysicalProgram);
                assert!(
                    !has_ime_context(&emulator),
                    "emulator IME context must be detached"
                );
                assert!(
                    has_ime_context(&text),
                    "other windows must keep their IME context"
                );
            }
            self.passed = true;
            event_loop.exit();
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }
    let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
    let mut probe = Probe::default();
    event_loop.run_app(&mut probe).unwrap();
    assert!(probe.passed);
}
