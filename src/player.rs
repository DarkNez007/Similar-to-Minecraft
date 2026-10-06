use crate::chunk::world_lookup_block;
use crate::states::{ActiveWorld, AppState};
use crate::world::LoadedChunks;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), grab_cursor)
            .add_systems(OnExit(AppState::InGame), release_cursor)
            .add_systems(
                Update,
                (
                    mouse_look,
                    toggle_flight,
                    apply_physics,
                    toggle_pause,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(Update, toggle_pause.run_if(in_state(AppState::Paused)));
    }
}

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct Velocity(pub Vec3);

#[derive(Component, Default)]
pub struct FlightState {
    /// Летит ли игрок сейчас
    pub is_flying: bool,
    /// Время последнего одиночного нажатия пробела (для детекта двойного)
    pub last_space_press: Option<f32>,
    /// Сколько секунд считается "двойным нажатием"
    pub double_press_window: f32,
    /// Активна ли сейчас вертикальная составляющая полёта (пробел/шифт зажат)
    pub flying_up: bool,
    pub flying_down: bool,
}

const WALK_SPEED: f32 = 8.0;
const FLY_SPEED: f32 = 12.0;
const FLY_VERTICAL_SPEED: f32 = 8.0;
const GRAVITY: f32 = 25.0;
const JUMP_SPEED: f32 = 9.0;
const MOUSE_SENSITIVITY: f32 = 0.003;

/// Размеры игрока (AABB). Ноги на y=translation.y, глаза на y+EYE_HEIGHT
const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;    // высота глаз от ног

/// Размер шага для разбиения движения на подшаги (антитуннелирование)
const MAX_STEP: f32 = 0.2;

fn grab_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
}

fn release_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn mouse_look(
    mut motion: EventReader<MouseMotion>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta.length_squared() < 1e-6 { return; }

    if let Ok(mut t) = q.get_single_mut() {
        let (mut yaw, mut pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
        yaw -= delta.x * MOUSE_SENSITIVITY;
        pitch = (pitch - delta.y * MOUSE_SENSITIVITY).clamp(-1.5, 1.5);
        t.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

/// Отслеживает двойное нажатие пробела и переключает режим полёта.
/// Также помечает, зажат ли пробел/шифт для вертикального движения.
fn toggle_flight(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<&mut FlightState, With<Player>>,
) {
    let Ok(mut flight) = q.get_single_mut() else { return };
    let now = time.elapsed_secs();

    // Обновляем флаги вертикального движения каждый кадр (работают только в полёте)
    flight.flying_up = flight.is_flying && keys.pressed(KeyCode::Space);
    flight.flying_down = flight.is_flying && keys.pressed(KeyCode::ShiftLeft);

    // Реагируем только на момент нажатия (не удержания)
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    // Проверяем: было ли предыдущее нажатие недавно?
    let is_double = match flight.last_space_press {
        Some(t) => (now - t) < flight.double_press_window,
        None => false,
    };

    if is_double {
        // Двойное нажатие — переключаем полёт
        flight.is_flying = !flight.is_flying;
        flight.last_space_press = None; // сбрасываем, чтобы третье нажатие не сработало сразу
    } else {
        // Одиночное — запоминаем время
        flight.last_space_press = Some(now);
    }
}

fn apply_physics(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    active: Option<Res<ActiveWorld>>,
    chunks: Option<Res<LoadedChunks>>,
    mut q: Query<(&mut Transform, &mut Velocity, &FlightState), With<Player>>,
) {
    let Ok((mut t, mut vel, flight)) = q.get_single_mut() else { return };

    let dt = time.delta_secs();

    // === 1. Горизонтальное желаемое движение ===
    let forward = *t.forward();
    let right = *t.right();
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { dir += forward; }
    if keys.pressed(KeyCode::KeyS) { dir -= forward; }
    if keys.pressed(KeyCode::KeyA) { dir -= right; }
    if keys.pressed(KeyCode::KeyD) { dir += right; }

    let speed = if flight.is_flying { FLY_SPEED } else { WALK_SPEED };

    let mut flat = Vec3::new(dir.x, 0.0, dir.z);
    if flat.length_squared() > 0.0 {
        flat = flat.normalize() * speed;
    }

    // === 2. Вертикальная скорость ===
    if flight.is_flying {
        // В полёте гравитации нет, а пробел/шифт задают вертикальную скорость напрямую
        let mut vy = 0.0;
        if flight.flying_up { vy += FLY_VERTICAL_SPEED; }
        if flight.flying_down { vy -= FLY_VERTICAL_SPEED; }
        vel.0 = Vec3::new(flat.x, vy, flat.z);
    } else {
        // Применяем гравитацию к вертикальной скорости
        vel.0.y -= GRAVITY * dt;
        // Ограничиваем скорость падения, чтобы не протыкать блоки
        vel.0.y = vel.0.y.max(-40.0);

        // Прыжок: применяем ТОЛЬКО если стоим на земле (vy ≈ 0 и ноги на поверхности)
        // Для простоты — если vy близко к 0 и нажали пробел
        if keys.just_pressed(KeyCode::Space) && vel.0.y.abs() < 0.5 {
            vel.0.y = JUMP_SPEED;
        }

        // Горизонтальная скорость — напрямую задаём (простая модель без инерции)
        vel.0.x = flat.x;
        vel.0.z = flat.z;
    }

    // === 3. Движение с коллизиями ===
    let delta = vel.0 * dt;

    // Разбиваем на подшаги, чтобы не туннелировать через тонкие препятствия
    let steps = (delta.length() / MAX_STEP).ceil().max(1.0) as i32;
    let step_delta = delta / steps as f32;

    for _ in 0..steps {
        // По осям раздельно — это классический подход для AABB-коллизий
        try_move_axis(&mut t.translation, step_delta.x, Vec3::X, active.as_deref(), chunks.as_deref());
        try_move_axis(&mut t.translation, step_delta.y, Vec3::Y, active.as_deref(), chunks.as_deref());
        try_move_axis(&mut t.translation, step_delta.z, Vec3::Z, active.as_deref(), chunks.as_deref());
    }

    // Если по Y мы упёрлись (стоим или ударились головой) — обнуляем вертикальную скорость
    // Определим это по факту: если хотели сдвинуться, но позиция не изменилась.
    // Проще: если игрок на земле, обнулим vy, если он отрицательный.
    if vel.0.y < 0.0 && is_on_ground(&t.translation, active.as_deref(), chunks.as_deref()) {
        vel.0.y = 0.0;
    }
    if vel.0.y > 0.0 && is_head_blocked(&t.translation, active.as_deref(), chunks.as_deref()) {
        vel.0.y = 0.0;
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) { return; }
    match state.get() {
        AppState::InGame => next.set(AppState::Paused),
        AppState::Paused => next.set(AppState::InGame),
        _ => {}
    }
}

/// Проверяет, есть ли твёрдый блок в точке.
fn is_solid_at(
    x: i32, y: i32, z: i32,
    active: Option<&ActiveWorld>,
    chunks: Option<&LoadedChunks>,
) -> bool {
    let (Some(active), Some(chunks)) = (active, chunks) else { return false };
    let Some(gen) = active.gen.as_ref() else { return false };
    world_lookup_block(gen, &chunks.data, &chunks.all_trees, x, y, z).is_solid()
}

/// Проверяет, пересекается ли AABB игрока (ноги в `pos`) с твёрдым блоком.
/// `pos` — мировая позиция ног игрока (центр по X/Z).
fn collides(
    eye_pos: Vec3,
    active: Option<&ActiveWorld>,
    chunks: Option<&LoadedChunks>,
) -> bool {
    let half = PLAYER_WIDTH / 2.0;
    let feet_y = eye_pos.y - EYE_HEIGHT;
    let min = Vec3::new(eye_pos.x - half, feet_y, eye_pos.z - half);
    let max = Vec3::new(eye_pos.x + half, feet_y + PLAYER_HEIGHT, eye_pos.z + half);

    let min_bx = min.x.floor() as i32;
    let max_bx = (max.x - 1e-4).floor() as i32;
    let min_by = min.y.floor() as i32;
    let max_by = (max.y - 1e-4).floor() as i32;
    let min_bz = min.z.floor() as i32;
    let max_bz = (max.z - 1e-4).floor() as i32;

    for bx in min_bx..=max_bx {
        for by in min_by..=max_by {
            for bz in min_bz..=max_bz {
                if is_solid_at(bx, by, bz, active, chunks) {
                    return true;
                }
            }
        }
    }
    false
}

/// Пытается сдвинуть `pos` вдоль одной оси на `amount`. Если после сдвига
/// возникает коллизия — сдвиг отменяется.
fn try_move_axis(
    pos: &mut Vec3,
    amount: f32,
    axis: Vec3,
    active: Option<&ActiveWorld>,
    chunks: Option<&LoadedChunks>,
) {
    if amount.abs() < 1e-6 { return; }
    let original = *pos;
    *pos += axis * amount;
    if collides(*pos, active, chunks) {
        *pos = original;
    }
}

/// Стоит ли игрок на земле (под ногами твёрдый блок)?
fn is_on_ground(
    pos: &Vec3,
    active: Option<&ActiveWorld>,
    chunks: Option<&LoadedChunks>,
) -> bool {
    // Проверяем блок прямо под AABB игрока
    let test_pos = Vec3::new(pos.x, pos.y - 0.05, pos.z);
    collides(test_pos, active, chunks)
}

/// Упирается ли голова в блок (сверху AABB твёрдый блок)?
fn is_head_blocked(
    pos: &Vec3,
    active: Option<&ActiveWorld>,
    chunks: Option<&LoadedChunks>,
) -> bool {
    let test_pos = Vec3::new(pos.x, pos.y + 0.05, pos.z);
    collides(test_pos, active, chunks)
}