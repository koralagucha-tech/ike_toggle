use core::sync::atomic::{AtomicBool, Ordering};

use skyline::install_hooks;
use smash::app::lua_bind::*;
use smash::app::{utility, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;
use smashline::*;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

// Mesh names must match the mesh groups in Ike's model (model.numdlb)
// EXACTLY, including capitalization. Change these if your model differs.
const MESH_EARS: &str = "Ears";
const MESH_TAIL: &str = "Tail";
const MESH_SHIRT: &str = "Shirt";

// Taunt that flips the toggle (same motion names as the original code).
const TOGGLE_TAUNT_L: &str = "appeal_lw_l";
const TOGGLE_TAUNT_R: &str = "appeal_lw_r";

// One flag per player slot, so two Ikes don't share the same toggle.
// false = normal outfit, true = Ears + Tail visible, Shirt hidden.
#[allow(clippy::declare_interior_mutable_const)]
const OFF: AtomicBool = AtomicBool::new(false);
static TOGGLE_STATE: [AtomicBool; 8] = [OFF; 8];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

unsafe fn is_ike(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    // change_motion is also called for weapons/items, so make sure
    // this is really a fighter before checking the kind.
    utility::get_category(&mut *module_accessor) == *BATTLE_OBJECT_CATEGORY_FIGHTER
        && utility::get_kind(&mut *module_accessor) == *FIGHTER_KIND_IKE
}

unsafe fn entry_id(module_accessor: *mut BattleObjectModuleAccessor) -> usize {
    (WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize) & 7
}

unsafe fn apply_meshes(module_accessor: *mut BattleObjectModuleAccessor, on: bool) {
    ModelModule::set_mesh_visibility(module_accessor, Hash40::new(MESH_EARS), on);
    ModelModule::set_mesh_visibility(module_accessor, Hash40::new(MESH_TAIL), on);
    ModelModule::set_mesh_visibility(module_accessor, Hash40::new(MESH_SHIRT), !on);
}

// ---------------------------------------------------------------------------
// Hook: detect the taunt and flip the toggle
// ---------------------------------------------------------------------------

#[skyline::hook(replace = MotionModule::change_motion)]
unsafe fn change_motion_hook(
    module_accessor: *mut BattleObjectModuleAccessor,
    motion_kind: Hash40,
    start_frame: f32,
    frame_speed_mult: f32,
    unk1: bool,
    unk2: f32,
    unk3: bool,
    unk4: bool,
) -> u64 {
    if is_ike(module_accessor)
        && (motion_kind.hash == Hash40::new(TOGGLE_TAUNT_L).hash
            || motion_kind.hash == Hash40::new(TOGGLE_TAUNT_R).hash)
    {
        let flag = &TOGGLE_STATE[entry_id(module_accessor)];
        let new_state = !flag.load(Ordering::Relaxed);
        flag.store(new_state, Ordering::Relaxed);
        apply_meshes(module_accessor, new_state);
    }

    // Call the original MotionModule::change_motion
    original!()(
        module_accessor,
        motion_kind,
        start_frame,
        frame_speed_mult,
        unk1,
        unk2,
        unk3,
        unk4,
    )
}

// ---------------------------------------------------------------------------
// Per-frame: re-apply visibility
//
// The game resets mesh visibility on respawn, on some motion changes, etc.,
// so we keep enforcing the toggle state every frame for Ike.
// ---------------------------------------------------------------------------

unsafe extern "C" fn ike_frame(fighter: &mut L2CFighterCommon) {
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(fighter.lua_state_agent);
    let on = TOGGLE_STATE[entry_id(module_accessor)].load(Ordering::Relaxed);
    apply_meshes(module_accessor, on);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[skyline::main(name = "ike_toggle")]
pub fn main() {
    install_hooks!(change_motion_hook);
    Agent::new("ike").on_line(Main, ike_frame).install();
}
