use livesplit_core::TimerPhase;
use smithay_client_toolkit::{
    default_environment,
    environment::{Environment, SimpleGlobal},
    new_default_environment,
    reexports::{
        calloop::{self, EventLoop},
        client::protocol::*,
        client::{Display, Main},
        protocols::wlr::unstable::layer_shell::v1::client::{
            zwlr_layer_shell_v1, zwlr_layer_surface_v1,
        },
    },
    shm::AutoMemPool,
    WaylandSource,
};

use std::{
    cell::Cell,
    convert::TryInto,
    error::Error,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{wl_split_timer::WlSplitTimer, TimerDisplay};

default_environment!(Env,
    fields = [
        layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    ],
    singles = [
        zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell
    ],
);

pub struct App<'a> {
    timer: Arc<Mutex<WlSplitTimer>>,
    surface: Surface,
    display: Display,
    event_loop: EventLoop<'a, ()>,
}

impl App<'_> {
    pub fn new(timer: WlSplitTimer) -> Self {
        let (env, display, queue) =
            new_default_environment!(Env, fields = [layer_shell: SimpleGlobal::new(),])
                .expect("Initial roundtrip failed!");
        let event_loop = calloop::EventLoop::<()>::try_new().unwrap();
        WaylandSource::new(queue)
            .quick_insert(event_loop.handle())
            .unwrap();
        let height: u32 = (timer.segments().len() * 50).try_into().unwrap();
        let surface = Surface::new(&env, None, (400, height));
        Self {
            timer: Arc::new(Mutex::new(timer)),
            surface,
            display,
            event_loop,
        }
    }
}

impl TimerDisplay for App<'_> {
    fn run(&mut self) -> Result<bool, Box<dyn Error>> {
        loop {
            let timer = self.timer.lock().unwrap();
            if timer.exit {
                break;
            }
            drop(timer);
            let mut redraw = false;
            match self.surface.handle_events() {
                Event::Close => break,
                Event::Redraw => redraw = true,
                Event::Idle => {}
            }
            if redraw || self.timer().lock().unwrap().timer().current_phase() == TimerPhase::Running
            {
                self.surface.draw(&self.timer);
            }
            self.display.flush().unwrap();
            self.event_loop
                .dispatch(Duration::from_millis(33), &mut ())
                .unwrap();
        }
        Ok(true)
    }

    fn timer(&self) -> &Arc<Mutex<WlSplitTimer>> {
        &self.timer
    }
}

#[derive(PartialEq, Copy, Clone)]
enum RenderEvent {
    Configure { width: u32, height: u32 },
    Closed,
}

enum Event {
    Close,
    Redraw,
    Idle,
}

struct Surface {
    surface: wl_surface::WlSurface,
    layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    next_render_event: Rc<Cell<Option<RenderEvent>>>,
    pool: AutoMemPool,
    dimensions: (u32, u32),
}

impl Surface {
    fn new(
        env: &Environment<Env>,
        output: Option<&wl_output::WlOutput>,
        dimensions: (u32, u32),
    ) -> Self {
        let pool = env
            .create_auto_pool()
            .expect("Failed to create memory pool");
        let layer_shell = env.require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();
        let surface = env.create_surface().detach();
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            output,
            zwlr_layer_shell_v1::Layer::Overlay,
            crate::app_name!().to_owned(),
        );

        layer_surface.set_size(dimensions.0, dimensions.1);
        layer_surface.set_margin(12, 0, 0, 12);
        // Anchor to the top left corner of the output
        layer_surface
            .set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);

        let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
        let next_render_event_handle = Rc::clone(&next_render_event);
        layer_surface.quick_assign(move |layer_surface, event, _| {
            match (event, next_render_event_handle.get()) {
                (zwlr_layer_surface_v1::Event::Closed, _) => {
                    next_render_event_handle.set(Some(RenderEvent::Closed));
                }
                (
                    zwlr_layer_surface_v1::Event::Configure {
                        serial,
                        width,
                        height,
                    },
                    next,
                ) if next != Some(RenderEvent::Closed) => {
                    layer_surface.ack_configure(serial);
                    next_render_event_handle.set(Some(RenderEvent::Configure { width, height }));
                }
                (_, _) => {}
            }
        });

        // Commit so that the server will send a configure event
        surface.commit();

        Self {
            surface,
            layer_surface,
            next_render_event,
            pool,
            dimensions: (0, 0),
        }
    }

    fn handle_events(&mut self) -> Event {
        match self.next_render_event.take() {
            Some(RenderEvent::Closed) => Event::Close,
            Some(RenderEvent::Configure { width, height }) => {
                self.dimensions = (width, height);
                Event::Redraw
            }
            None => Event::Idle,
        }
    }

    fn draw(&mut self, timer: &Arc<Mutex<WlSplitTimer>>) {
        let stride = 4 * self.dimensions.0 as i32;
        let width = self.dimensions.0 as i32;
        let height = self.dimensions.1 as i32;

        let (canvas, buffer) = if let Ok((canvas, buffer)) =
            self.pool
                .buffer(width, height, stride, wl_shm::Format::Argb8888)
        {
            (canvas, buffer)
        } else {
            return;
        };

        let timer = timer.lock().unwrap();
        for dst_pixel in canvas.chunks_exact_mut(4) {
            let pixel = if timer.timer().current_phase() == TimerPhase::Running {
                0x1100ff00u32.to_ne_bytes()
            } else {
                0x11ff0000u32.to_ne_bytes()
            };
            dst_pixel[0] = pixel[0];
            dst_pixel[1] = pixel[1];
            dst_pixel[2] = pixel[2];
            dst_pixel[3] = pixel[3];
        }
        drop(timer);

        self.surface.attach(Some(&buffer), 0, 0);
        self.surface
            .damage_buffer(0, 0, width as i32, height as i32);

        self.surface.commit();
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
