import bpy, bmesh, struct, math
from mathutils import Vector
source = r'D:\EMTYPYIE\Kohlrausch\work\cherry-mx-model\scene.gltf'
output = r'D:\EMTYPYIE\Kohlrausch\ui\assets\cherry_mx_switch_3pin.stl'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=source)
names = {
 'Mid Switch_Blue Plastic_0',
 'Mid Bottom_Mid Bottom_0',
 'Mid Top_Mid Top_0',
 'Mid LED_LED_0',
 'Mid LED_Metal_0',
 'Mid Spring_Metal_0',
}
parts = [o for o in bpy.context.scene.objects if o.type == 'MESH' and o.name in names]
missing = names - {o.name for o in parts}
if missing:
 raise RuntimeError('Missing Cherry MX model parts: ' + ', '.join(sorted(missing)))
verts=[]; faces=[]
for obj in parts:
 start=len(verts)
 for vertex in obj.data.vertices:
  world=obj.matrix_world @ vertex.co
  verts.append(((world.x+0.018)*1000.0, world.y*1000.0, world.z*1000.0))
 for polygon in obj.data.polygons:
  faces.append(tuple(start+i for i in polygon.vertices))
mesh=bpy.data.meshes.new('CherryMX3Pin')
mesh.from_pydata(verts,[],faces); mesh.update()
obj=bpy.data.objects.new('Cherry MX 3-pin switch',mesh); bpy.context.collection.objects.link(obj)
bpy.context.view_layer.objects.active=obj; obj.select_set(True)
# Keep the shell, coil, and terminals readable while reducing the downloaded high-poly coil mesh.
dec=obj.modifiers.new('Switch display detail reduction','DECIMATE'); dec.ratio=0.28
bpy.ops.object.modifier_apply(modifier=dec.name)
bm=bmesh.new(); bm.from_mesh(obj.data)
bmesh.ops.triangulate(bm, faces=list(bm.faces))
bm.to_mesh(obj.data); bm.free(); obj.data.update()
triangles=[]
for polygon in obj.data.polygons:
 if len(polygon.vertices)!=3: continue
 points=[obj.data.vertices[i].co for i in polygon.vertices]
 normal=(points[1]-points[0]).cross(points[2]-points[0])
 if normal.length: normal.normalize()
 triangles.append((normal,points))
with open(output,'wb') as f:
 header=b'Cherry MX 3-pin switch assembly derived from Sketchfab model'
 f.write(header[:80].ljust(80,b' ')); f.write(struct.pack('<I',len(triangles)))
 for normal,points in triangles:
  f.write(struct.pack('<12fH',normal.x,normal.y,normal.z,*[component for p in points for component in p],0))
mins=[min(v[i] for v in verts) for i in range(3)]
maxs=[max(v[i] for v in verts) for i in range(3)]
open(r'D:\EMTYPYIE\Kohlrausch\work\cherry_mx_export_info.txt','w').write(f"parts={len(parts)}\ncomponents={sorted(names)}\nsource_faces={sum(len(o.data.polygons) for o in parts)}\nexport_triangles={len(triangles)}\nbounds_mm={mins}..{maxs}\n")
