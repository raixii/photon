import bpy
import os
import sys
import json
import itertools
import gzip
from math import sqrt
from mathutils import Matrix, Vector, Euler, Quaternion

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

def convert_color(c):
    return [c.r, c.g, c.b]

def convert_prop_array(arr):
    return list(arr)

def convert_vector(v):
    return list(v)

def convert_matrix(m):
    return [[c for c in r] for r in m]

def rename(name):
    return name.replace(" ", "_").lower()

def main():
    if "--" in sys.argv:
        args = sys.argv[sys.argv.index("--") + 1:]
    else:
        args = []

    if "--pretty" in args:
        pretty = True
        args.remove("--pretty")
    else:
        pretty = False

    if len(args) >= 1:
        outfile = args[0]
    else:
        outfile = "-"

    export_path = bpy.data.filepath + ".json.gz"
    eprint("Exporting to: " + export_path)
    eprint()

    out_objects = {} 
    depsgraph = bpy.context.evaluated_depsgraph_get()
    for object_inst in depsgraph.object_instances:
        object = object_inst.object
        eprint(f">>> Object {object.type} {object.name}")

        out_object = dict()
        out_object["name"] = object.name
        out_object["type"] = object.type

        if object.type == "MESH":
            out_object["matrix"] = convert_matrix(object.matrix_world)

            mesh = object.to_mesh()
            mesh.calc_loop_triangles()
            eprint(f"\t{len(mesh.loop_triangles)} triangles")
            uv_layer = mesh.uv_layers.active.data
            triangles = []
            for t in mesh.loop_triangles:
                for v in t.vertices:
                    triangles.append({
                        "p": convert_vector(mesh.vertices[v].co),
                        "n": convert_vector(mesh.vertices[v].normal if t.use_smooth else t.normal),
                        "t": convert_vector(uv_layer[v].uv)
                    })
            object.to_mesh_clear()
            out_object["triangles"] = triangles

            material = object.active_material
            out_material = dict()
            out_object["material"] = out_material
            out_material["name"] = material.name
            out_material["nodes"] = {}
            for (node_name, node) in material.node_tree.nodes.items():
                out_node = {}
                out_node["name"] = node_name
                out_node["type"] = node.type
                for prefix, items in (("in", node.inputs.items()), ("out", node.outputs.items())):
                    for (name, value) in items:
                        key = prefix + "_" + rename(name)
                        if value.type == "VALUE":
                            out_node[key] = { "type": "VALUE", "value": value.default_value }
                        elif value.type == "RGBA":
                            out_node[key] = { "type": "VALUE", "value": convert_prop_array(value.default_value) }
                        elif value.type == "VECTOR":
                            out_node[key] = { "type": "VALUE", "value": convert_vector(value.default_value) }
                        elif value.type == "SHADER":
                            out_node[key] = { "type": "VALUE", "value": None }
                        else:
                            eprint("\tUnknown type", value.type, "of input socket", name)
                if node.type == "TEX_IMAGE":
                    out_node["interpolation"] = node.interpolation
                    out_node["projection"] = node.projection
                    out_node["extension"] = node.extension
                    out_node["source"] = node.image.source
                    out_node["filepath"] = node.image.filepath
                    out_node["colorspace"] = node.image.colorspace_settings.name
                out_material["nodes"][out_node["name"]] = out_node
            for link in material.node_tree.links:
                out_material["nodes"][link.to_node.name]["in_" + rename(link.to_socket.name)] = {
                    "type": "LINK",
                    "from_node": link.from_node.name,
                    "from_socket": rename(link.from_socket.name),
                }
        elif object.type == "LIGHT":
            out_object["lamp_type"] = object.data.type
            out_object["color"] = convert_color(object.data.color)
            out_object["power"] = object.data.energy
            out_object["specular"] = object.data.specular_factor
            out_object["radius"] = object.data.shadow_soft_size
            out_object["attenuation"] = [0.00111109, 0.0, 1.0]
            out_object["matrix"] = convert_matrix(object.matrix_world)
        elif object.type == "CAMERA":
            out_object["matrix"] = convert_matrix(object.matrix_world)
            out_object["xfov"] = object.data.angle_x
            out_object["yfov"] = object.data.angle_y
            out_object["znear"] = object.data.clip_start
            out_object["zfar"] = object.data.clip_end
            out_object["camera_type"] = object.data.type

        out_objects[object.name] = out_object

    out = {
        "objects": out_objects,
    }

    json_str = json.dumps(out, check_circular=False, indent=(2 if pretty else None))
    if outfile == "-":
        print(json_str)
    else:
        with open(outfile, "w", encoding="UTF-8") as fp:
            fp.write(json_str)

main()
