use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys;

struct Ball {
    x: i32,
    y: i32,
    radius: i32, // ボールの半径
    dx: i32,     // ボールのx軸進行速度
    dy: i32,     // ボールのy軸進行速度
}

impl Ball {
    fn draw(&self, ctx: &web_sys::CanvasRenderingContext2d) {
        ctx.begin_path();
        let _ = ctx.arc(
            self.x as f64,
            self.y as f64,
            self.radius as f64,
            0.0,
            std::f64::consts::PI * 2.0,
        );
        ctx.set_fill_style(&JsValue::from_str("#0095DD"));
        ctx.fill();
        ctx.close_path();
    }
}

struct Brick {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    status: bool, // falseならば描画しない
}

impl Brick {
    fn draw(&self, ctx: &web_sys::CanvasRenderingContext2d) {
        if self.status {
            ctx.begin_path();
            ctx.rect(
                self.x as f64,
                self.y as f64,
                self.width as f64,
                self.height as f64,
            );
            ctx.set_fill_style(&JsValue::from_str("#0095DD"));
            ctx.fill();
            ctx.close_path();
        }
    }
}

struct Bricks {
    inner: Vec<Brick>,
}

impl Bricks {
    pub fn new() -> Self {
        let mut bricks: Vec<Brick> = vec![];

        let row = 3;
        let column = 5;
        let width = 75;
        let height = 20;
        let padding = 10;
        let offset_top = 30;
        let offset_left = 30;

        for r in 0..row {
            for c in 0..column {
                let brick = Brick {
                    x: c * (width + padding) + offset_left,
                    y: r * (height + padding) + offset_top,
                    width,
                    height,
                    status: true,
                };

                bricks.push(brick);
            }
        }

        Self { inner: bricks }
    }

    fn draw(&self, ctx: &web_sys::CanvasRenderingContext2d) {
        self.inner.iter().for_each(|b| b.draw(ctx));
    }
}

struct Paddle {
    x: i32,
    //y: i32, // 縦軸方向には移動しないため、yは不要
    width: i32,
    height: i32,
}

impl Paddle {
    fn draw(&self, ctx: &web_sys::CanvasRenderingContext2d) {
        let canvas_height = ctx.canvas().unwrap().height() as i32;
        ctx.begin_path();
        ctx.rect(
            self.x as f64,
            (canvas_height - &self.height) as f64,
            self.width as f64,
            self.height as f64,
        );
        ctx.set_fill_style(&JsValue::from_str("#0095DD"));
        ctx.fill();
        ctx.close_path();
    }
}

struct UserInput {
    keyboard_right: bool,
    keyboard_left: bool,
    mouse_x: i32,
    mouse_y: i32, // パドルの移動は横方向のみなので、今回は使わない
}

impl UserInput {
    fn set_keydoard_right(&mut self, press: bool) {
        self.keyboard_right = press;
    }
    fn set_keydoard_left(&mut self, press: bool) {
        self.keyboard_left = press;
    }
    fn set_mouse_position(&mut self, x: i32, y: i32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
}

struct Game {
    canvas_context: web_sys::CanvasRenderingContext2d,
    canvas_width: i32,
    canvas_height: i32,
    ball: Ball,
    paddle: Paddle,
    bricks: Bricks,
    score: u16,
    lives: u16,
    user_input: UserInput,
    game_loop_closure: Option<Closure<dyn FnMut()>>, // ゲームループクローザ
    game_loop_interval_handle: Option<i32>,          // ゲームループのハンドル
}

impl Game {
    pub fn new() -> Self {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let canvas = document.get_element_by_id("myCanvas").unwrap();
        let canvas = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        let canvas_context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;

        let ball = Ball {
            x: canvas_width / 2,
            y: canvas_height - 30,
            radius: 10,
            dx: 2,
            dy: -2,
        };

        let paddle = Paddle {
            width: 75,
            height: 10,
            x: (canvas_width - 10) / 2,
        };

        let user_input = UserInput {
            keyboard_right: false,
            keyboard_left: false,
            mouse_x: 0,
            mouse_y: 0,
        };

        let bricks = Bricks::new();

        Self {
            canvas_context,
            canvas_width,
            canvas_height,
            ball,
            paddle,
            bricks,
            score: 0,
            lives: 3,
            user_input,
            game_loop_closure: None,
            game_loop_interval_handle: None,
        }
    }

    // ゲームループ
    fn game_loop(&mut self) {
        // 画面を初期化
        self.canvas_context.clear_rect(
            0.0,
            0.0,
            self.canvas_width as f64,
            self.canvas_height as f64,
        );

        // ボールの描画
        self.ball.draw(&self.canvas_context);
        // パドルの描画
        self.paddle.draw(&self.canvas_context);
        // ブロックの描画
        self.bricks.draw(&self.canvas_context);
        // スコアの描画
        self.draw_score();
        // ライフの描画
        self.draw_lives();

        // ブロックとの衝突
        self.collision_detection();

        // ボールの移動先
        let moved_ball_x = self.ball.x.saturating_add(self.ball.dx);
        let moved_ball_y = self.ball.y.saturating_add(self.ball.dy);

        // ボールと左右の壁の衝突
        if moved_ball_x > self.canvas_width - self.ball.radius || moved_ball_x < self.ball.radius {
            self.ball.dx = -self.ball.dx;
        }

        // ボールと上下の壁の衝突
        if moved_ball_y < self.ball.radius {
            self.ball.dy = -self.ball.dy;
        } else if moved_ball_y > self.canvas_height - self.ball.radius {
            // ボールがパドルに当たった場合は反射
            if self.ball.x > self.paddle.x && self.ball.x < self.paddle.x + self.paddle.width {
                self.ball.dy = -self.ball.dy;
            } else {
                // ボールが画面下部に当たった場合はライフを減らす
                self.lives = self.lives.saturating_sub(1);

                // ライフが0になったらゲームオーバー
                if self.lives == 0 {
                    let window = web_sys::window().unwrap();
                    let document = window.document().unwrap();

                    window.alert_with_message("GAME OVER").unwrap();
                    document.location().unwrap().reload().unwrap();
                } else {
                    // ライフがまだ残っている場合はボールとパドルを初期位置に戻す
                    self.ball.x = self.canvas_width / 2;
                    self.ball.y = self.canvas_height - 30;
                    self.ball.dx = 2;
                    self.ball.dy = -2;
                    self.paddle.x = (self.canvas_width - self.paddle.width) / 2;
                }
            }
        }

        // パドルの左右移動
        if self.user_input.keyboard_right == true {
            self.paddle.x = std::cmp::min(
                self.paddle.x.saturating_add(7),
                self.canvas_width.saturating_sub(self.paddle.width),
            );
        } else if self.user_input.keyboard_left == true {
            self.paddle.x = std::cmp::max(self.paddle.x.saturating_sub(7), 0);
        }

        // パドルをマウスに追従させる
        let canvas = self.canvas_context.canvas().unwrap();
        let relative_x = self.user_input.mouse_x.saturating_sub(canvas.offset_left());
        if relative_x > 0 && relative_x < self.canvas_width {
            self.paddle.x = relative_x.saturating_sub(self.paddle.width / 2);
        }

        // ボールの移動
        self.ball.x = self.ball.x.saturating_add(self.ball.dx);
        self.ball.y = self.ball.y.saturating_add(self.ball.dy);

        self.start_game_loop();
    }

    fn collision_detection(&mut self) {
        for brick in &mut self.bricks.inner {
            if brick.status
                && self.ball.x > brick.x
                && self.ball.x < brick.x + brick.width
                && self.ball.y > brick.y
                && self.ball.y < brick.y + brick.height
            {
                // ボールがブロックに当たったらボールを反射
                self.ball.dy = -self.ball.dy;

                // ボールが当たったブロックを消すためにstatusをfalseにする
                brick.status = false;

                // スコアを加算
                self.score += 1;
            }
        }

        // スコアがブロックの数と同じになったらゲームクリア
        if self.score == self.bricks.inner.len() as u16 {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();

            window
                .alert_with_message("YOU WIN, CONGRATULATIONS!")
                .unwrap();
            document.location().unwrap().reload().unwrap();
        }
    }

    // スコア描画
    fn draw_score(&self) {
        self.canvas_context.set_font("16px Arial");
        self.canvas_context
            .set_fill_style(&JsValue::from_str("#0095DD"));
        self.canvas_context
            .fill_text(&format!("Score: {}", self.score), 8.0, 20.0)
            .unwrap();
    }

    // ライフ描画
    fn draw_lives(&self) {
        self.canvas_context.set_font("16px Arial");
        self.canvas_context
            .set_fill_style(&JsValue::from_str("#0095DD"));
        self.canvas_context
            .fill_text(
                &format!("Lives: {}", self.lives),
                self.canvas_width as f64 - 65.0,
                20.0,
            )
            .unwrap();
    }

    // メソッドではなく、関連関数なので Game::set_game_loop_and_start() として呼び出す
    // 引数には自分自身を Rc<RefCell<>> で包んだものを渡す
    pub fn set_game_loop_and_start(game: Rc<RefCell<Self>>) {
        let cloned_game = game.clone();
        let mut game_borrow = game.borrow_mut();

        game_borrow.set_game_loop(move || cloned_game.borrow_mut().game_loop());
        game_borrow.start_game_loop();
    }

    fn set_game_loop<F: 'static>(&mut self, f: F)
    where
        F: FnMut(),
    {
        self.game_loop_closure = Some(Closure::new(f));
    }

    fn start_game_loop(&mut self) {
        // クロージャの参照を取り出す
        let closure = self.game_loop_closure.as_ref().unwrap();
        let window = web_sys::window().unwrap();

        let handle = window.request_animation_frame(closure.as_ref().unchecked_ref());

        self.game_loop_interval_handle = Some(handle.unwrap());
    }

    // メソッドではなく、関連関数なので Game::set_input_event() として呼び出す
    // 引数には自分自身を Rc<RefCell<>> で包んだものを渡す
    pub fn set_input_event(game: Rc<RefCell<Self>>) {
        let game_key_down = game.clone();
        let game_key_up = game.clone();
        let game_mouse_move = game.clone();
        let document = web_sys::window().unwrap().document().unwrap();

        let closure = Closure::new(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut g = game_key_down.borrow_mut();
            if event.code() == "Right" || event.code() == "ArrowRight" {
                g.user_input.set_keydoard_right(true);
            } else if event.code() == "Left" || event.code() == "ArrowLeft" {
                g.user_input.set_keydoard_left(true);
            }
        }) as Box<dyn FnMut(_)>);

        document.set_onkeydown(Some(&closure.as_ref().unchecked_ref()));
        // forget()するとRust側はdropされるが、into_js_value()されてブラウザ側に残る
        closure.forget();

        let closure = Closure::new(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut g = game_key_up.borrow_mut();
            if event.code() == "Right" || event.code() == "ArrowRight" {
                g.user_input.set_keydoard_right(false);
            } else if event.code() == "Left" || event.code() == "ArrowLeft" {
                g.user_input.set_keydoard_left(false);
            }
        }) as Box<dyn FnMut(_)>);

        document.set_onkeyup(Some(&closure.as_ref().unchecked_ref()));
        // forget()するとRust側はdropされるが、into_js_value()されてブラウザ側に残る
        closure.forget();

        let closure = Closure::new(Box::new(move |event: web_sys::MouseEvent| {
            let mut g = game_mouse_move.borrow_mut();
            g.user_input
                .set_mouse_position(event.client_x(), event.client_y());
        }) as Box<dyn FnMut(_)>);

        document.set_onmousemove(Some(&closure.as_ref().unchecked_ref()));
        // forget()するとRust側はdropされるが、into_js_value()されてブラウザ側に残る
        closure.forget();
    }
}

#[wasm_bindgen]
pub fn run() {
    let game = Game::new();
    let game = Rc::new(RefCell::new(game));
    Game::set_game_loop_and_start(game.clone());
    Game::set_input_event(game.clone());
}
