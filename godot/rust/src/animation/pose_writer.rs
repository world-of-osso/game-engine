//! Bone pose writes as direct method-bind ptrcalls on one live skeleton.
//!
//! godot-rust checks an object's liveness before every engine call (an `ObjectDB` lookup
//! under a global spin lock). A 40-player crowd made 12k-16k such checks per frame in
//! `set_bone_pose_*` alone, 28-33% of all checks; the lookup was 6-8% of main-thread
//! time. A writer checks liveness once, then calls the same engine methods with the
//! same arguments.
use godot::builtin::{Quaternion, StringName, Vector3};
use godot::classes::Skeleton3D;
use godot::obj::Gd;
use godot::sys;
use std::sync::OnceLock;

/// `Skeleton3D` method binds of the Godot 4.7 API (extension_api.json hashes).
struct PoseBinds {
    position: sys::GDExtensionMethodBindPtr,
    rotation: sys::GDExtensionMethodBindPtr,
    scale: sys::GDExtensionMethodBindPtr,
}

// SAFETY: method binds are immutable engine globals that live as long as the engine.
unsafe impl Send for PoseBinds {}
// SAFETY: as above; the binds are only read.
unsafe impl Sync for PoseBinds {}

fn pose_binds() -> &'static PoseBinds {
    static BINDS: OnceLock<PoseBinds> = OnceLock::new();
    BINDS.get_or_init(|| PoseBinds {
        position: method_bind("set_bone_pose_position", 1530502735),
        rotation: method_bind("set_bone_pose_rotation", 2823819782),
        scale: method_bind("set_bone_pose_scale", 1530502735),
    })
}

fn method_bind(method: &str, hash: i64) -> sys::GDExtensionMethodBindPtr {
    let class = StringName::from("Skeleton3D");
    let name = StringName::from(method);
    // SAFETY: the interface is initialized once an extension object exists; both names
    // are valid StringNames for the duration of the call.
    let bind = unsafe {
        sys::interface_fn!(classdb_get_method_bind)(class.string_sys(), name.string_sys(), hash)
    };
    assert!(
        !bind.is_null(),
        "Skeleton3D::{method} has no bind with hash {hash}"
    );
    bind
}

/// Writes bone pose components to a skeleton that was alive when the writer was made.
/// The writer borrows the skeleton mutably, so it cannot be freed through this handle
/// while writing, and the pose setters run no script.
pub(super) struct PoseWriter<'a> {
    object: sys::GDExtensionObjectPtr,
    binds: &'static PoseBinds,
    _skeleton: std::marker::PhantomData<&'a mut Gd<Skeleton3D>>,
}

impl<'a> PoseWriter<'a> {
    pub fn new(skeleton: &'a mut Gd<Skeleton3D>) -> Self {
        assert!(
            skeleton.is_instance_valid(),
            "Skeleton3D pose write after the skeleton was freed"
        );
        Self {
            object: skeleton.obj_sys(),
            binds: pose_binds(),
            _skeleton: std::marker::PhantomData,
        }
    }

    pub fn position(&mut self, bone: i32, position: Vector3) {
        self.call(self.binds.position, bone, &position);
    }

    pub fn rotation(&mut self, bone: i32, rotation: Quaternion) {
        self.call(self.binds.rotation, bone, &rotation);
    }

    pub fn scale(&mut self, bone: i32, scale: Vector3) {
        self.call(self.binds.scale, bone, &scale);
    }

    /// `bind(bone, value)` with ptrcall arguments: an int is passed as int64, a Vector3 or
    /// Quaternion as its own bytes.
    fn call<T>(&self, bind: sys::GDExtensionMethodBindPtr, bone: i32, value: &T) {
        let bone = i64::from(bone);
        let args: [sys::GDExtensionConstTypePtr; 2] = [
            std::ptr::from_ref(&bone).cast(),
            std::ptr::from_ref(value).cast(),
        ];
        // SAFETY: `object` is a live Skeleton3D (checked in `new`, borrowed since); `bind`
        // is the Skeleton3D method taking (int, T) and returning nothing, so no return
        // value is written.
        unsafe {
            sys::interface_fn!(object_method_bind_ptrcall)(
                bind,
                self.object,
                args.as_ptr(),
                std::ptr::null_mut(),
            );
        }
    }
}
