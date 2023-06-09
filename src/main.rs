// Augmented Audio: Audio libraries and applications
// Copyright (c) 2022 Pedro Tacla Yamada
//
// The MIT License (MIT)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//! An example of sending commands from another thread.
//! This is useful when you want to have some kind of
//! generated content (like here), or some task that just
//! takes a long time but don't want to block the main thread
//! (waiting on an http request, some cpu intensive work etc.)

use std::thread;
use std::time::Duration;

use druid::widget::prelude::*;
use druid::{AppLauncher, Color, Point, Selector, Target, WidgetExt, WindowDesc};

use audio_garbage_collector::GarbageCollector;
use audio_processor_standalone::audio_processor_start;

use crate::buffer_analyser::BufferAnalyserProcessor;
use atomic_queue::Queue;
use basedrop::Shared;
use druid::kurbo::BezPath;

mod buffer_analyser;

// If you want to submit commands to an event sink you have to give it some kind
// of ID. The selector is that, it also assures the accompanying data-type is correct.
// look at the docs for `Selector` for more detail.
const DRAW_AUDIO: Selector<Vec<f32>> = Selector::new("event-example.draw_audio");

pub fn main() {
    let window = WindowDesc::new(make_ui()).title("External Event Demo");

    let launcher = AppLauncher::with_window(window);
    let event_sink = launcher.get_external_handle();

    let garbage_collector = GarbageCollector::default();
    let processor = BufferAnalyserProcessor::new(garbage_collector.handle());
    let queue_handle = processor.queue();
    let _audio_streams = audio_processor_start(processor);
    thread::spawn(move || generate_audio_updates(event_sink, queue_handle));

    launcher
        .launch(AudioData(Vec::new()))
        .expect("launch failed");
}

fn generate_audio_updates(event_sink: druid::ExtEventSink, queue_handle: Shared<Queue<f32>>) {
    let mut buffer = Vec::with_capacity(5 * 4410);
    buffer.resize(5 * 4410, 0.0);
    let buffer_size = buffer.len();
    let mut position = 0;

    loop {
        while let Some(sample) = queue_handle.pop() {
            buffer[position % buffer_size] = sample;
            position += 1;
        }

        if event_sink
            .submit_command(DRAW_AUDIO, buffer.clone(), Target::Auto)
            .is_err()
        {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[derive(Clone)]
struct AudioData(Vec<f32>);

impl Data for AudioData {
    fn same(&self, _other: &Self) -> bool {
        false
    }
}

/// A widget that displays a color.
struct AudioWave {}

impl Widget<AudioData> for AudioWave {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut AudioData, _env: &Env) {
        match event {
            // This is where we handle our command.
            Event::Command(cmd) if cmd.is(DRAW_AUDIO) => {
                // We don't do much data processing in the `event` method.
                // All we really do is just set the data. This causes a call
                // to `update` which requests a paint. You can also request a paint
                // during the event, but this should be reserved for changes to self.
                // For changes to `Data` always make `update` do the paint requesting.
                *data = AudioData(cmd.get_unchecked(DRAW_AUDIO).clone());
            }
            _ => (),
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AudioData,
        _: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AudioData, _data: &AudioData, _: &Env) {
        ctx.request_paint()
    }

    fn layout(&mut self, _: &mut LayoutCtx, bc: &BoxConstraints, _: &AudioData, _: &Env) -> Size {
        bc.max()
    }

    // This is of course super slow due to using CoreGraphics
    fn paint(&mut self, ctx: &mut PaintCtx, data: &AudioData, _env: &Env) {
        // ctx.clear(Color::BLACK);
        let AudioData(data) = data;
        if data.is_empty() {
            return;
        }

        let size = ctx.size();
        let mut prev = data[0];
        let mut index = 0;

        let mut shape = BezPath::new();
        let num_points = data.len();
        let width = size.width;
        let step = ((num_points as f64) / width) as usize;
        while index < data.len() {
            let item = data[index];
            let f_index = index as f64;
            let x_coord = (f_index / data.len() as f64) * size.width;
            let y_coord = (prev as f64) * size.height / 2.0 + size.height / 2.0;
            shape.move_to(Point::new(x_coord, y_coord));

            let mut draw = |item| {
                let x2_coord = ((f_index + 1.0) / data.len() as f64) * size.width;
                let y2_coord = (item as f64) * size.height / 2.0 + size.height / 2.0;
                shape.line_to(Point::new(x2_coord, y2_coord));
            };

            draw(item);
            draw(-item);

            prev = item;
            index += step;
        }
        ctx.stroke(shape, &Color::RED, 3.0);
    }
}

fn make_ui() -> impl Widget<AudioData> {
    AudioWave {}.expand().padding(10.0).center()
}
