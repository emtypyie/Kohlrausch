import bpy, json
path = r'D:\EMTYPYIE\Kohlrausch\work\cherry-mx-model\scene.gltf'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=path)
items=[]
for o in bpy.context.scene.objects:
 if o.type == 'MESH':
  bpy.context.view_layer.update()
  pts=[o.matrix_world @ v.co for v in o.data.vertices]
  if pts:
   items.append({'name':o.name,'vertices':len(o.data.vertices),'polygons':len(o.data.polygons),'min':[min(p[i] for p in pts) for i in range(3)],'max':[max(p[i] for p in pts) for i in range(3)]})
open(r'D:\EMTYPYIE\Kohlrausch\work\gltf_mesh_summary.json','w').write(json.dumps(items,indent=2))
