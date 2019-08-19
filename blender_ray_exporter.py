import bpy
import os
import sys
import json
import itertools
from math import sqrt
from mathutils import Matrix, Vector, Euler, Quaternion

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
    export_path = bpy.data.filepath + ".json"
    print("Exporting to: " + export_path)
    print()

    out_objects = {} 
    depsgraph = bpy.context.evaluated_depsgraph_get()
    for object_inst in depsgraph.object_instances:
        object = object_inst.object
        print(f">>> Object {object.type} {object.name}")

        out_object = dict()
        out_object["name"] = object.name
        out_object["type"] = object.type

        if object.type == "MESH":
            out_object["matrix"] = convert_matrix(object.matrix_world)

            mesh = object.to_mesh()
            mesh.calc_loop_triangles()
            print(f"\t{len(mesh.loop_triangles)} triangles")
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
            material_output_node = material.node_tree.nodes['Material Output']
            surface_node = material_output_node.inputs["Surface"].links[0].from_node
            out_material["type"] = surface_node.type
            for (in_name, in_value) in surface_node.inputs.items():
                if in_value.type == "VALUE":
                    out_material[rename(in_name)] = in_value.default_value
                elif in_value.type == "RGBA":
                    out_material[rename(in_name)] = convert_prop_array(in_value.default_value)
                elif in_value.type == "VECTOR":
                    out_material[rename(in_name)] = convert_vector(in_value.default_value)
                else:
                    print("\tUnknown type in material node", in_value.type)
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

    with open(export_path, "w", encoding="UTF-8") as fp:
        json.dump(out, fp, check_circular=False)

main()
