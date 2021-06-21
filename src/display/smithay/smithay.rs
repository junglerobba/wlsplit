use andrew::Canvas;
use livesplit_core::{Segment, TimeSpan, TimerPhase};
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

use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};

use crate::{config::Config, time_format::TimeFormat, wl_split_timer::WlSplitTimer, TimerDisplay};

default_environment!(Env,
    fields = [
        layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    ],
    singles = [
        zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell
    ],
);

type Damage = [usize; 4];

#[derive(Debug)]
pub enum SplitColor {
    Gain,
    Loss,
    Gold,
}

pub struct App<'a> {
    timer: Arc<Mutex<WlSplitTimer>>,
    surface: Surface,
    display: Display,
    event_loop: EventLoop<'a, ()>,
}

impl App<'_> {
    pub fn new(timer: WlSplitTimer, config: &Config) -> Self {
        let (env, display, queue) =
            new_default_environment!(Env, fields = [layer_shell: SimpleGlobal::new(),])
                .expect("Initial roundtrip failed!");
        let event_loop = calloop::EventLoop::<()>::try_new().unwrap();
        WaylandSource::new(queue)
            .quick_insert(event_loop.handle())
            .unwrap();

        let height = get_total_height(timer.segments().len(), config.text_size, config.padding_v);
        let surface = Surface::new(&env, None, (config.width as u32, height as u32), config);
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
        let mut extra_frame = false;
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

            let timer_running =
                self.timer().lock().unwrap().timer().current_phase() == TimerPhase::Running;
            if redraw || timer_running || extra_frame {
                self.surface.draw(&self.timer);
            }
            extra_frame = timer_running;
            self.display.flush().unwrap();
            self.event_loop
                .dispatch(Duration::from_millis(33), &mut ())
                .unwrap();
            std::thread::sleep(Duration::from_millis(33));
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

#[derive(Debug, Copy, Clone)]
struct RenderProperties {
    text_height: usize,
    padding_h: usize,
    padding_v: usize,
    background_color: [u8; 4],
    background_opacity: u8,
    font_color: [u8; 4],
    font_color_gain: [u8; 4],
    font_color_loss: [u8; 4],
    font_color_gold: [u8; 4],
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
    current_scale: i32,
    scale_handle: Rc<Cell<i32>>,
    current_split: Option<usize>,
    font_data: Vec<u8>,
    render_properties: RenderProperties,
}

impl Surface {
    fn new(
        env: &Environment<Env>,
        output: Option<&wl_output::WlOutput>,
        dimensions: (u32, u32),
        config: &Config,
    ) -> Self {
        let pool = env
            .create_auto_pool()
            .expect("Failed to create memory pool");
        let layer_shell = env.require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();
        let scale = Rc::new(Cell::new(1));
        let scale_handle = Rc::clone(&scale);
        let surface = env
            .create_surface_with_scale_callback(move |dpi, _, _| {
                scale.set(dpi);
            })
            .detach();
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            output,
            zwlr_layer_shell_v1::Layer::Overlay,
            crate::app_name!().to_owned(),
        );

        layer_surface.set_size(dimensions.0, dimensions.1);
        layer_surface.set_margin(
            config.margin.0,
            config.margin.1,
            config.margin.2,
            config.margin.3,
        );
        // Anchor to the top left corner of the output
        let mut anchor = zwlr_layer_surface_v1::Anchor::all();
        anchor.set(
            zwlr_layer_surface_v1::Anchor::Top,
            config.anchor.contains("top"),
        );
        anchor.set(
            zwlr_layer_surface_v1::Anchor::Bottom,
            config.anchor.contains("bottom"),
        );
        anchor.set(
            zwlr_layer_surface_v1::Anchor::Left,
            config.anchor.contains("left"),
        );
        anchor.set(
            zwlr_layer_surface_v1::Anchor::Right,
            config.anchor.contains("right"),
        );
        layer_surface.set_anchor(anchor);

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

        let family_name = config
            .font_family
            .clone()
            .map_or_else(|| FamilyName::Monospace, FamilyName::Title);
        let font = SystemSource::new()
            .select_best_match(&[family_name], &Properties::new())
            .unwrap()
            .load()
            .unwrap();
        let font_data = font.copy_font_data().unwrap().to_vec();
        Self {
            surface,
            layer_surface,
            next_render_event,
            pool,
            dimensions: (0, 0),
            current_scale: 1,
            scale_handle,
            current_split: None,
            font_data,
            render_properties: RenderProperties {
                text_height: config.text_size,
                padding_h: config.padding_h,
                padding_v: config.padding_v,
                background_color: [
                    255,
                    config.background_color[0],
                    config.background_color[1],
                    config.background_color[2],
                ],
                background_opacity: config.background_opacity,
                font_color: config.font_color,
                font_color_gain: config.font_color_gain,
                font_color_loss: config.font_color_loss,
                font_color_gold: config.font_color_gold,
            },
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
        let scale = self.scale_handle.get();
        if self.current_scale != scale {
            self.current_scale = scale;
            self.surface.set_buffer_scale(scale);
            println!("Scale set to {}", scale);
            // Force full redraw
            self.current_split = None;
        }
        let stride = 4 * self.dimensions.0 as i32 * scale;
        let width = self.dimensions.0 as i32 * scale;
        let height = self.dimensions.1 as i32 * scale;

        let scale = scale as usize;
        let (pixels, buffer) = if let Ok((canvas, buffer)) =
            self.pool
                .buffer(width, height, stride, wl_shm::Format::Argb8888)
        {
            (canvas, buffer)
        } else {
            return;
        };

        let timer = timer.lock().unwrap();
        let mut canvas = andrew::Canvas::new(
            pixels,
            width as usize,
            height as usize,
            stride as usize,
            andrew::Endian::native(),
        );
        let mut damage: Vec<Damage> = Vec::new();
        match self.current_split {
            Some(previous_split) => {
                let current_split = if let Some(index) = timer.current_segment_index() {
                    index
                } else {
                    self.current_split = None;
                    return;
                };
                if previous_split != current_split {
                    damage.push(Surface::draw_segment_title(
                        previous_split,
                        false,
                        &timer.segments()[previous_split],
                        &mut canvas,
                        &self.font_data,
                        &self.render_properties,
                        scale,
                    ));
                    damage.push(Surface::draw_segment_title(
                        current_split,
                        true,
                        &timer.current_segment().unwrap(),
                        &mut canvas,
                        &self.font_data,
                        &self.render_properties,
                        scale,
                    ));
                    damage.push(Surface::draw_segment_time(
                        previous_split,
                        &timer.segments()[previous_split],
                        false,
                        &mut canvas,
                        &self.font_data,
                        width as usize,
                        &timer,
                        &self.render_properties,
                        scale,
                    ));
                    damage.push(Surface::draw_attempts_counter(
                        timer.run().attempt_count() as usize,
                        &self.font_data,
                        &self.render_properties,
                        width as usize,
                        &mut canvas,
                        scale,
                    ));
                    let best_segment = timer.get_personal_best_segment_time(previous_split);
                    let current_segment = timer.get_segment_time(previous_split);
                    let diff = diff_time(
                        current_segment.map(|msecs| TimeSpan::from_milliseconds(msecs as f64)),
                        best_segment.and_then(|segment| segment.real_time),
                    );
                    let mut previous_segment_render_properties = self.render_properties.clone();
                    previous_segment_render_properties.font_color = match diff.1 {
                        SplitColor::Gain => self.render_properties.font_color_gain,
                        SplitColor::Loss => self.render_properties.font_color_loss,
                        SplitColor::Gold => self.render_properties.font_color_gold,
                    };
                    damage.push(Surface::draw_additional_info(
                        &mut canvas,
                        timer.segments().len() + 3,
                        &previous_segment_render_properties,
                        &self.font_data,
                        width as usize,
                        "Previous segment",
                        &diff.0,
                        scale,
                    ))
                }
                damage.push(Surface::draw_segment_time(
                    current_split,
                    &timer.current_segment().unwrap(),
                    true,
                    &mut canvas,
                    &self.font_data,
                    width as usize,
                    &timer,
                    &self.render_properties,
                    scale,
                ));
            }
            None => {
                damage.push([0, 0, width as usize, height as usize]);
                canvas.clear();
                canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
                    (0, 0),
                    (width as usize, height as usize),
                    None,
                    Some(self.render_properties.background_color),
                ));
                let title = format!("{} ({})", timer.game_name(), timer.category_name());
                canvas.draw(&andrew::text::Text::new(
                    (
                        self.render_properties.padding_h * scale,
                        self.render_properties.padding_v * scale,
                    ),
                    self.render_properties.font_color,
                    &self.font_data,
                    (self.render_properties.text_height * scale) as f32,
                    1.0,
                    title,
                ));

                Surface::draw_attempts_counter(
                    timer.run().attempt_count() as usize,
                    &self.font_data,
                    &self.render_properties,
                    width as usize,
                    &mut canvas,
                    scale,
                );

                for (i, segment) in timer.segments().iter().enumerate() {
                    let current_segment = timer.current_segment_index().unwrap_or(0);
                    self.current_split = Some(current_segment);
                    Surface::draw_segment_title(
                        i,
                        i == current_segment,
                        segment,
                        &mut canvas,
                        &self.font_data,
                        &self.render_properties,
                        scale,
                    );
                    Surface::draw_segment_time(
                        i,
                        segment,
                        i == current_segment,
                        &mut canvas,
                        &self.font_data,
                        width as usize,
                        &timer,
                        &self.render_properties,
                        scale,
                    );
                }

                Surface::draw_additional_info(
                    &mut canvas,
                    timer.segments().len() + 2,
                    &self.render_properties,
                    &self.font_data,
                    width as usize,
                    "Sum of best segments",
                    &TimeFormat::default()
                        .format_time(timer.best_possible_time().try_into().unwrap(), false),
                    scale,
                );
            }
        }
        let mut current_time = andrew::text::Text::new(
            (0, 0),
            self.render_properties.font_color,
            &self.font_data,
            (self.render_properties.text_height * scale) as f32 * 1.2,
            1.0,
            &timer.time().map_or_else(
                || "/".to_string(),
                |time| {
                    TimeFormat::default()
                        .format_time(time.to_duration().num_milliseconds() as u128, false)
                },
            ),
        );
        let pos = (
            width as usize - current_time.get_width() - self.render_properties.padding_h * scale,
            (2 * self.render_properties.padding_v
                + ((timer.segments().len() + 1)
                    * (self.render_properties.text_height + self.render_properties.padding_v)))
                * scale,
        );

        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            pos,
            (
                current_time.get_width() + self.render_properties.padding_h,
                (self.render_properties.text_height + self.render_properties.padding_v) * scale,
            ),
            None,
            Some(self.render_properties.background_color),
        ));
        current_time.pos = pos;
        canvas.draw(&current_time);
        damage.push([
            current_time.pos.0,
            current_time.pos.1,
            current_time.get_width() + self.render_properties.padding_h,
            (self.render_properties.text_height + self.render_properties.padding_v) * scale,
        ]);
        self.current_split = timer.current_segment_index();
        drop(timer);

        // Ugly workaround for transparency
        for dst_pixel in pixels.chunks_exact_mut(4) {
            if dst_pixel[0] == self.render_properties.background_color[1]
                && dst_pixel[1] == self.render_properties.background_color[2]
                && dst_pixel[2] == self.render_properties.background_color[3]
            {
                dst_pixel[3] = self.render_properties.background_opacity;
            }
        }
        self.surface.attach(Some(&buffer), 0, 0);
        for damage in damage {
            self.surface.damage_buffer(
                damage[0] as i32,
                damage[1] as i32,
                damage[2] as i32,
                damage[3] as i32,
            );
        }

        self.surface.commit();
    }
    fn draw_segment_title(
        index: usize,
        current: bool,
        segment: &Segment,
        canvas: &mut Canvas,
        font_data: &[u8],
        render_properties: &RenderProperties,
        scale: usize,
    ) -> Damage {
        let name = format!("> {}", segment.name().to_string());
        let pos = (
            render_properties.padding_h * scale,
            (render_properties.padding_v
                + ((index + 1) * (render_properties.text_height + render_properties.padding_v)))
                * scale,
        );
        let mut title = andrew::text::Text::new(
            pos,
            render_properties.font_color,
            &font_data,
            (render_properties.text_height * scale) as f32,
            1.0,
            &name,
        );
        let damage: Damage = [
            title.pos.0,
            title.pos.1,
            (title.get_width() + render_properties.padding_h) * scale,
            (render_properties.text_height + render_properties.padding_v) * scale,
        ];
        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            title.pos,
            (
                (title.get_width() + render_properties.padding_h) * scale,
                (render_properties.text_height + render_properties.padding_v) * scale,
            ),
            None,
            Some(render_properties.background_color),
        ));

        if !current {
            title.text = String::from(name.strip_prefix("> ").unwrap());
        }

        canvas.draw(&title);
        damage
    }

    fn draw_segment_time(
        index: usize,
        segment: &Segment,
        current: bool,
        canvas: &mut Canvas,
        font_data: &[u8],
        width: usize,
        timer: &WlSplitTimer,
        render_properties: &RenderProperties,
        scale: usize,
    ) -> Damage {
        let timestamp = if let Some(time) = segment.personal_best_split_time().real_time {
            Some(time)
        } else if segment.segment_history().iter().len() == 0 {
            segment.split_time().real_time
        } else {
            None
        };
        let mut time = andrew::text::Text::new(
            (0, 0),
            render_properties.font_color,
            &font_data,
            (render_properties.text_height * scale) as f32,
            1.0,
            &timestamp.map_or_else(
                || "/".to_string(),
                |time| {
                    TimeFormat::default()
                        .format_time(time.to_duration().num_milliseconds() as u128, false)
                },
            ),
        );
        time.pos = (
            width as usize - time.get_width() - render_properties.padding_h * scale,
            (render_properties.padding_v
                + ((index + 1) * (render_properties.text_height + render_properties.padding_v)))
                * scale,
        );

        let diff_timestamp = {
            let mut diff = diff_time(
                if current {
                    timer.time()
                } else {
                    segment.split_time().real_time
                },
                timer.segments()[index].personal_best_split_time().real_time,
            );
            let gold = if let (Some(split), Some(pb)) = (
                timer.get_segment_time(index),
                timer.segments()[index].best_segment_time().real_time,
            ) {
                split < pb.to_duration().num_milliseconds().try_into().unwrap()
            } else {
                false
            };
            if !current && gold {
                diff.1 = SplitColor::Gold;
            }
            diff
        };
        let mut diff = andrew::text::Text::new(
            (0, 0),
            match diff_timestamp.1 {
                SplitColor::Gain => render_properties.font_color_gain,
                SplitColor::Loss => render_properties.font_color_loss,
                SplitColor::Gold => render_properties.font_color_gold,
            },
            &font_data,
            (render_properties.text_height * scale) as f32 * 0.9,
            1.0,
            "-:--:--.---",
        );
        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            time.pos,
            (
                (time.get_width() + render_properties.padding_h) * scale,
                (render_properties.text_height + render_properties.padding_v) * scale,
            ),
            None,
            Some(render_properties.background_color),
        ));
        let diff_damage_pos = (
            width as usize
                - time.get_width()
                - diff.get_width()
                - render_properties.padding_h * 4 * scale,
            (render_properties.padding_v
                + ((index + 1) * (render_properties.text_height + render_properties.padding_v))
                + (render_properties.text_height / 20))
                * scale,
        );
        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            diff_damage_pos,
            (
                (diff.get_width() + render_properties.padding_h) * scale,
                (render_properties.text_height + render_properties.padding_v) * scale,
            ),
            None,
            Some(render_properties.background_color),
        ));
        let damage: Damage = [
            diff_damage_pos.0,
            diff_damage_pos.1,
            diff.get_width() + time.get_width() + 6 * render_properties.padding_h * scale,
            (render_properties.text_height + render_properties.padding_v) * scale,
        ];
        diff.text = diff_timestamp.0;
        diff.pos = (
            width as usize
                - time.get_width()
                - diff.get_width()
                - render_properties.padding_h * 4 * scale,
            (render_properties.padding_v
                + ((index + 1) * (render_properties.text_height + render_properties.padding_v))
                + (render_properties.text_height / 20))
                * scale,
        );
        canvas.draw(&time);
        canvas.draw(&diff);

        damage
    }

    fn draw_attempts_counter(
        attempt_count: usize,
        font_data: &[u8],
        render_properties: &RenderProperties,
        width: usize,
        canvas: &mut Canvas,
        scale: usize,
    ) -> Damage {
        let mut attempts = andrew::text::Text::new(
            (0, 0),
            render_properties.font_color,
            &font_data,
            (render_properties.text_height * scale) as f32,
            1.0,
            attempt_count.to_string(),
        );
        attempts.pos = (
            (width as usize - attempts.get_width() - render_properties.padding_h) * scale,
            render_properties.padding_v * scale,
        );
        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            attempts.pos,
            (
                (attempts.get_width() + render_properties.padding_h) * scale,
                (render_properties.text_height + render_properties.padding_v) * scale,
            ),
            None,
            Some(render_properties.background_color),
        ));
        canvas.draw(&attempts);
        [
            attempts.pos.0,
            attempts.pos.1,
            attempts.get_width() + render_properties.padding_h,
            render_properties.text_height + render_properties.padding_v,
        ]
    }

    fn draw_additional_info(
        canvas: &mut Canvas,
        offset: usize,
        render_properties: &RenderProperties,
        font_data: &[u8],
        width: usize,
        text_left: &str,
        text_right: &str,
        scale: usize,
    ) -> Damage {
        let text_left = andrew::text::Text::new(
            (
                render_properties.padding_h * scale,
                (2 * render_properties.padding_v
                    + ((offset) * (render_properties.text_height + render_properties.padding_v)))
                    * scale,
            ),
            render_properties.font_color,
            &font_data,
            (render_properties.text_height * scale) as f32,
            1.0,
            text_left,
        );
        let mut text_right = andrew::text::Text::new(
            (0, 0),
            render_properties.font_color,
            &font_data,
            (render_properties.text_height * scale) as f32,
            1.0,
            text_right,
        );
        text_right.pos = (
            width as usize - text_right.get_width() - render_properties.padding_h * scale,
            (2 * render_properties.padding_v
                + ((offset) * (render_properties.text_height + render_properties.padding_v)))
                * scale,
        );
        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            text_left.pos,
            (
                text_left.get_width() + render_properties.padding_h * scale,
                (render_properties.text_height + render_properties.padding_v) * scale,
            ),
            None,
            Some(render_properties.background_color),
        ));
        canvas.draw(&andrew::shapes::rectangle::Rectangle::new(
            text_right.pos,
            (
                text_right.get_width() + render_properties.padding_h * scale,
                (render_properties.text_height + render_properties.padding_v) * scale,
            ),
            None,
            Some(render_properties.background_color),
        ));
        canvas.draw(&text_left);
        canvas.draw(&text_right);
        [
            text_left.pos.0,
            text_right.pos.1,
            width as usize,
            (render_properties.text_height + render_properties.padding_v) * scale,
        ]
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}

fn diff_time(time: Option<TimeSpan>, best: Option<TimeSpan>) -> (String, SplitColor) {
    if let (Some(time), Some(best)) = (time, best) {
        let time = time.to_duration().num_milliseconds();
        let best = best.to_duration().num_milliseconds();
        let negative = best > time;
        let diff = if negative { best - time } else { time - best } as u128;
        return (
            TimeFormat::for_diff().format_time(diff, negative),
            if negative {
                SplitColor::Gain
            } else {
                SplitColor::Loss
            },
        );
    }
    ("".to_string(), SplitColor::Loss)
}

fn get_total_height(len: usize, text_height: usize, padding_v: usize) -> usize {
    (len + 5) * (text_height + padding_v)
}
