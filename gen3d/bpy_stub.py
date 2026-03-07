"""
Stub bpy (Blender Python) module.

The Hunyuan3D-2.1 texture pipeline imports `bpy` at the top of
DifferentiableRenderer/mesh_utils.py, but the only bpy-dependent function
(convert_obj_to_glb) is never called by our server — we pass save_glb=False
and use create_glb_with_pbr_materials instead.

This stub allows the import to succeed without a real Blender installation.
Any actual bpy operation will raise RuntimeError.
"""


class _Stub:
    """Lazy stub that returns itself for any attribute access."""

    def __getattr__(self, name):
        return _Stub()

    def __call__(self, *args, **kwargs):
        raise RuntimeError(
            "bpy stub: Blender is not installed. "
            "convert_obj_to_glb requires a real Blender install."
        )

    def __lt__(self, other):
        return False

    def __le__(self, other):
        return False

    def __gt__(self, other):
        return False

    def __ge__(self, other):
        return False

    def __iter__(self):
        return iter([])

    def __bool__(self):
        return False

    def __len__(self):
        return 0


data = _Stub()
context = _Stub()
ops = _Stub()
app = _Stub()
version = (0, 0, 0)
