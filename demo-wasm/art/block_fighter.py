"""Original four-part block mannequin. No imported rig, mesh, texture or motion.
Blender --background --python block_fighter.py -- --output DIR [--render]
Blender --background DIR/block-fighter.blend --python block_fighter.py -- --check
Reference findings and deliberate simplifications: README.md.
"""
import argparse
import json
import math
from pathlib import Path
import sys
import bpy
from mathutils import Vector, Matrix, Quaternion
from bpy_extras.object_utils import world_to_camera_view

ap = argparse.ArgumentParser()
ap.add_argument('--output', default=str(Path(__file__).parent / 'generated'))
ap.add_argument('--render', action='store_true')
ap.add_argument('--clip', action='append', help='Render only a named repaired clip; still save the complete rig')
ap.add_argument('--check', action='store_true')
a = ap.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
OUT = Path(a.output)

if a.check:
    data = json.loads((Path(bpy.data.filepath).parent / 'manifest.json').read_text())
    rig = bpy.data.objects['Block mannequin']
    scene = bpy.context.scene
    assert scene.render.film_transparent and scene.render.image_settings.color_mode == 'RGBA'
    assert scene.camera.data.type == 'ORTHO'
    assert sorted(bpy.data.actions.keys()) == sorted(data['clips'])
    error = 0.0
    edge_error, leg_step, min_z = 0., 0., 1e9
    worst_leg = None
    meshes = [o for o in scene.objects if o.type == 'MESH']
    edges = {o.name:[(e.vertices[:], (o.data.vertices[e.vertices[0]].co-o.data.vertices[e.vertices[1]].co).length)
                     for e in o.data.edges] for o in meshes}
    for name, clip in data['clips'].items():
        action = bpy.data.actions[name]
        rig.animation_data.action = action
        rig.animation_data.action_slot = action.slots[0]
        for frame, expected in clip['witnesses'].items():
            scene.frame_set(int(frame))
            for bone, matrix in expected.items():
                actual = rig.pose.bones[bone].matrix
                error = max(error, max(abs(actual[r][c] - matrix[r][c]) for r in range(4) for c in range(4)))
        previous = {}
        for frame in range(1,clip['count']+1):
            scene.frame_set(frame)
            for bone in rig.pose.bones:
                if bone.name.startswith(('thigh.','shin.','foot.')):
                    rotation = bone.matrix.to_quaternion()
                    if bone.name in previous:
                        delta = math.degrees(rotation.rotation_difference(previous[bone.name]).angle)
                        step = min(delta,360-delta)
                        if step > leg_step:leg_step,worst_leg = step,(name,frame,bone.name)
                    previous[bone.name] = rotation
            dg = bpy.context.evaluated_depsgraph_get()
            for obj in meshes:
                evaluated = obj.evaluated_get(dg)
                geometry = evaluated.to_mesh()
                vertices = [evaluated.matrix_world@v.co for v in geometry.vertices]
                min_z = min(min_z, *[v.z for v in vertices])
                for (i,j),rest in edges[obj.name]:
                    edge_error = max(edge_error,abs((vertices[i]-vertices[j]).length/rest-1))
                evaluated.to_mesh_clear()
    assert error < 0.0001, error
    assert edge_error < .00001, ('Rigid mesh changed shape',edge_error)
    assert leg_step < 50, ('Leg pop',leg_step,worst_leg,'minimum_z',min_z)
    assert min_z > -.015, ('Geometry below floor',min_z)
    assert len([o for o in scene.objects if o.type == 'MESH']) == data['mesh_count']
    # Finite motion identity gate: these fail for the retired hook/hammer/side kick.
    def sample(name, frame=None):
        action=bpy.data.actions[name]
        rig.animation_data.action=action
        rig.animation_data.action_slot=action.slots[0]
        scene.frame_set(frame or data['clips'][name]['contacts'][0][0])
        return {b.name:(b.head.copy(),b.tail.copy()) for b in rig.pose.bones}
    guard=sample('idle',1); jab=sample('jab'); mp=sample('follow'); overhead=sample('heavy')
    assert jab['hand.rear'][1].x > jab['hand.lead'][1].x+.35, '5LP uses the far/left fist'
    assert mp['hand.rear'][1].x < jab['hand.rear'][1].x-.15, '5MP must stay compact'
    assert mp['hand.rear'][1].z < jab['hand.rear'][1].z-.15, '5MP body-punch height'
    for side in ('lead','rear'):
        assert (mp['foot.'+side][0]-guard['foot.'+side][0]).length < .0001, '5MP planted feet'
    assert overhead['hand.lead'][0].z < overhead['fore.lead'][0].z-.15, '6MP descends from the elbow'
    assert overhead['hand.rear'][0].z > overhead['hand.lead'][0].z+.30, '6MP is not a two-hand hammer'
    lk=sample('low_light'); mk=sample('low_medium'); hp=sample('low_heavy'); hk=sample('target')
    assert lk['foot.rear'][1].x > lk['foot.lead'][1].x+.50, '2LK far/left leg'
    assert mk['foot.lead'][1].x > mk['foot.rear'][1].x+.50, '2MK near/right leg'
    assert mk['hand.lead'][0].z < .15 and mk['hand.lead'][0].x < mk['pelvis'][0].x-.35, '2MK hand brace'
    assert hp['hand.lead'][1].z > hp['hand.rear'][1].z+.25, '2HP one-arm rising uppercut'
    assert hk['foot.lead'][1].z > hk['head'][0].z and hk['foot.rear'][0].z < .15, 'High turning kick, not waist-height side kick'
    for name in ('special','charged'):
        palm=sample(name)
        assert palm['hand.lead'][1].x > palm['hand.rear'][1].x+.10, (name,'dominant palm / supporting hand')
        assert palm['hand.lead'][1].z > palm['hand.rear'][1].z+.10, (name,'asymmetric Hashogeki')
    charge=sample('reload',13)
    assert charge['pelvis'][0].z > .78 and max(charge['hand.'+side][0].z for side in ('lead','rear')) < 1.3, 'Standing Denjin charge'
    print('RYU_MOTION_LANDMARKS_OK nine attacks and standing charge')
    print('SOURCE_READBACK_OK ' + json.dumps({'clips': list(data['clips']), 'max_matrix_error': error, 'max_edge_error':edge_error, 'max_leg_step_degrees':leg_step, 'minimum_z':min_z}))
    raise SystemExit(0)

OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
scene = bpy.context.scene
scene.render.engine = 'BLENDER_EEVEE'
scene.render.resolution_x = scene.render.resolution_y = 448
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGBA'
scene.render.film_transparent = True
scene.render.fps = 24
scene.world.color = (.24, .24, .24)
scene.view_settings.view_transform = 'Standard'
scene.view_settings.look = 'None'

materials = {}
for name, color in {'clay': (.55, .32, .17), 'teal': (.10, .34, .35), 'slate': (.14, .21, .29)}.items():
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get('Principled BSDF')
    bsdf.inputs['Base Color'].default_value = (*color, 1)
    bsdf.inputs['Roughness'].default_value = .85
    bsdf.inputs['Specular IOR Level'].default_value = .12
    materials[name] = mat

# The four trunk masses are pelvis, lower abdomen, upper abdomen and chest.
REST = {'root': ((0, 0, 0), (0, 0, .12), None)}
for name, lo, hi, parent in [('pelvis', 1.08, 1.22, 'root'), ('abdomen.low', 1.22, 1.36, 'pelvis'),
                           ('abdomen.high', 1.36, 1.50, 'abdomen.low'), ('chest', 1.50, 1.73, 'abdomen.high'),
                           ('neck', 1.73, 1.82, 'chest'), ('head', 1.82, 2.09, 'neck')]:
    REST[name] = ((0, 0, lo), (0, 0, hi), parent)
for side, s in [('lead', -1), ('rear', 1)]:
    shoulder, elbow, wrist = (0, s*.255, 1.66), (0, s*.44, 1.38), (0, s*.54, 1.11)
    hip, knee, ankle = (0, s*.12, 1.08), (0, s*.12, .58), (0, s*.12, .10)
    REST.update({f'upper.{side}': (shoulder, elbow, 'chest'), f'fore.{side}': (elbow, wrist, f'upper.{side}'),
                 f'hand.{side}': (wrist, (0, s*.54, .97), f'fore.{side}'),
                 f'thigh.{side}': (hip, knee, 'pelvis'), f'shin.{side}': (knee, ankle, f'thigh.{side}'),
                 f'foot.{side}': (ankle, (.18, s*.12, .10), f'shin.{side}')})
arm = bpy.data.armatures.new('Original block skeleton')
rig = bpy.data.objects.new('Block mannequin', arm)
scene.collection.objects.link(rig)
bpy.context.view_layer.objects.active = rig
rig.select_set(True)
bpy.ops.object.mode_set(mode='EDIT')
for name, (p, q, parent) in REST.items():
    bone = arm.edit_bones.new(name)
    bone.head, bone.tail = p, q
    if parent:
        bone.parent = arm.edit_bones[parent]
bpy.ops.object.mode_set(mode='OBJECT')
rig.show_in_front = True
for bone in rig.pose.bones:
    bone.rotation_mode = 'QUATERNION'
meshes = []

# Reuse the previous proof's rigid vertex-group/armature binding, not its styling.
def mesh(name, vertices, mat, bone):
    data = bpy.data.meshes.new(name)
    data.from_pydata(vertices, [], [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4),
                                   (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)])
    data.update()
    obj = bpy.data.objects.new(name, data)
    scene.collection.objects.link(obj)
    data.materials.append(materials[mat])
    group = obj.vertex_groups.new(name=bone)
    group.add(list(range(8)), 1, 'REPLACE')
    obj.modifiers.new('Rigid bone binding', 'ARMATURE').object = rig
    obj.parent = rig
    meshes.append(obj)


def block(name, bone, mat, bottom, top=None, start=.015, end=.985):
    p, q = (Vector(v) for v in REST[bone][:2])
    axis = (q-p).normalized()
    u = Vector((1, 0, 0))
    u = (u-axis*u.dot(axis)).normalized()
    v = axis.cross(u)
    vertices = []
    for t, dims in [(start, bottom), (end, top or bottom)]:
        c = p.lerp(q, t)
        vertices.extend([tuple(c+u*x*dims[0]/2+v*y*dims[1]/2) for x, y in [(-1,-1),(1,-1),(1,1),(-1,1)]])
    mesh(name, vertices, mat, bone)


def box(name, center, size, mat, bone):
    x, y, z = center
    dx, dy, dz = (s/2 for s in size)
    mesh(name, [(x+sx*dx, y+sy*dy, z+sz*dz) for sz in [-1,1] for sx,sy in [(-1,-1),(1,-1),(1,1),(-1,1)]], mat, bone)


block('Pelvis', 'pelvis', 'slate', (.27,.38), (.25,.32), -.38, 1)
block('Lower abdomen', 'abdomen.low', 'teal', (.25,.32), (.235,.30), 0, 1)
block('Upper abdomen', 'abdomen.high', 'teal', (.235,.30), (.25,.38), 0, 1)
block('Chest', 'chest', 'teal', (.25,.38), (.29,.49), 0, 1)
block('Neck', 'neck', 'clay', (.115,.115), start=0, end=1.02)
block('Plain head', 'head', 'clay', (.225,.22), start=0, end=1)
for side, s in [('lead',-1), ('rear',1)]:
    block('Upper arm '+side, 'upper.'+side, 'clay', (.14,.145), (.11,.12), 0, 1)
    block('Forearm '+side, 'fore.'+side, 'clay', (.115,.12), (.09,.095), 0, 1)
    block('Closed hand '+side, 'hand.'+side, 'clay', (.105,.115), (.125,.12), 0, 1)
    # ponytail: closed-fist proof only; add finger bones only for an open-hand move.
    box('Tucked thumb '+side, (.055,s*.54-s*.043,1.028), (.042,.053,.065), 'clay', 'hand.'+side)
    block('Thigh '+side, 'thigh.'+side, 'slate', (.19,.19), (.135,.14), 0, 1)
    block('Shin '+side, 'shin.'+side, 'slate', (.135,.14), (.09,.10), 0, 1)
    y = s*.12
    mesh('Low wedge foot '+side, [(-.075,y-.068,0),(.20,y-.068,0),(.20,y+.068,0),(-.075,y+.068,0),
                                  (-.075,y-.068,.13),(.20,y-.068,.052),(.20,y+.068,.052),(-.075,y+.068,.13)], 'slate', 'foot.'+side)

bpy.ops.object.camera_add(location=(3.6,-9,2.6))
cam = bpy.context.object
cam.name = 'Fixed orthographic game camera'
target = Vector((.23,0,1.02))
cam.rotation_euler = (target-cam.location).to_track_quat('-Z','Y').to_euler()
cam.data.type, cam.data.ortho_scale = 'ORTHO', 2.9
scene.camera = cam
for name, loc, energy, size in [('Key',(1.5,-4,5),650,4), ('Fill',(-3,-1,3),300,3)]:
    bpy.ops.object.light_add(type='AREA', location=loc)
    light = bpy.context.object
    light.name, light.data.energy, light.data.size = name, energy, size
    light.rotation_euler = (Vector((0,0,1))-light.location).to_track_quat('-Z','Y').to_euler()


def length(bone):
    return (Vector(REST[bone][1])-Vector(REST[bone][0])).length


def ik(p, q, pole, l1, l2):
    p, q, pole = map(Vector, (p,q,pole))
    delta = q-p
    distance = min(max(delta.length, abs(l1-l2)+.00001), l1+l2-.00001)
    direction = delta.normalized()
    q = p+direction*distance
    bend = pole-p
    bend = bend-direction*bend.dot(direction)
    assert bend.length > .0001, "IK pole aligned with the limb"
    bend.normalize()
    along = (l1*l1-l2*l2+distance*distance)/(2*distance)
    return p+direction*along+bend*math.sqrt(max(0,l1*l1-along*along)), q


def amount(frame, keys):
    for (a,x),(b,y) in zip(keys,keys[1:]):
        if a <= frame <= b:
            t = (frame-a)/(b-a)
            return x+(y-x)*t*t*(3-2*t)
    return keys[-1][1]


# Count, first contact (1-based), and editable motion envelope. Game timing is separate.
CLIPS = {
    'idle': (41,1,[]), 'crouch': (25,1,[]), 'walk': (25,13,[]), 'walkback': (25,13,[]),
    'jump': (9,9,[(0,0),(8,1)]), 'fall': (9,1,[(0,1),(8,0)]),
    'hitstun': (25,1,[(0,.6),(2,1),(5,1),(14,.35),(24,0)]),
    'blockstun': (13,1,[(0,1),(3,.85),(12,0)]),
    'blockstun_low': (13,1,[(0,1),(3,.85),(12,0)]),
    'jab': (25,9,[(0,0),(5,-.1),(8,1),(10,.98),(17,.08),(21,0),(24,0)]),
    'follow': (31,13,[(0,0),(7,-.30),(12,1),(17,1),(23,.10),(27,0),(30,0)]),
    'heavy': (41,19,[(0,0),(11,-.2),(18,1),(21,1),(33,.04),(40,0)]),
    'low_light': (25,9,[(0,0),(4,-.1),(8,1),(10,1),(18,.03),(24,0)]),
    'low_medium': (37,17,[(0,0),(8,-.15),(16,1),(19,1),(30,.03),(36,0)]),
    'low_heavy': (37,17,[(0,0),(8,-.12),(16,1),(19,1),(30,.03),(36,0)]),
    'target': (41,15,[(0,0),(5,0),(14,1),(17,1),(31,.1),(35,0),(40,0)]),
    'special': (29,11,[(0,0),(6,-.2),(10,1),(13,1),(22,.08),(28,0)]),
    'charged': (35,15,[(0,0),(9,-.4),(14,1),(17,1),(28,.08),(34,0)]),
    'finisher': (65,13,[]), 'multi': (41,9,[]),
    'reload': (25,13,[(0,0),(7,.7),(12,1),(17,.8),(24,0)]),
}
if a.clip and not set(a.clip) <= set(CLIPS):
    raise ValueError("Unknown render clip")
# Source contact holds; the consumer maps these to native hit-window boundaries.
CONTACTS = {'jab':[(9,11)], 'follow':[(13,15)], 'heavy':[(19,22)],
            'low_light':[(9,11)], 'low_medium':[(17,20)], 'low_heavy':[(17,20)],
            'target':[(15,18)], 'special':[(11,14)], 'charged':[(15,18)],
            'finisher':[(13,15),(29,31),(44,47)], 'multi':[(9,10),(23,24)]}
STRIKES = {'jab':['Closed hand rear'], 'follow':['Closed hand rear'],
           'heavy':['Closed hand lead'], 'low_light':['Low wedge foot rear'],
           'low_medium':['Low wedge foot lead'], 'low_heavy':['Closed hand lead'],
           'target':['Low wedge foot lead'], 'special':['Closed hand lead'], 'charged':['Closed hand lead'],
           'finisher':['Closed hand rear','Thigh lead','Closed hand lead'],
           'multi':['Closed hand rear','Closed hand lead']}
CLOSED = {'idle','crouch','walk','walkback','jab','follow','heavy','low_light','low_medium',
          'low_heavy','special','charged','finisher','multi','reload'}


def solve_leg(hip, ankle, side, turn=0):
    # A hinge axis, not a pole point that the foot can cross during a high turn.
    lateral=Quaternion((0,0,1),turn)@Vector((0,1,0))
    return ik(hip,ankle,hip+(ankle-hip).cross(lateral),length('thigh.'+side),length('shin.'+side))


def check_old_kick():
    # Regression of the reported high-kick trajectory, not just the replacement moves.
    previous, maximum = None, 0
    for f in range(35):
        q = max(0,amount(f,[(0,0),(6,-.18),(12,1),(15,1),(25,.15),(34,0)]))
        hip = Vector((-.06*q,0,.955-.04*q)) + Quaternion((0,0,1),.14-.5*q)@Vector((0,.12,0))
        ankle = Vector((amount(f,[(0,-.29),(6,.08),(12,1.02),(15,1.02),(25,.08),(34,-.29)]),.17,
                        amount(f,[(0,.10),(6,.80),(12,1.30),(15,1.30),(25,.78),(34,.10)])))
        knee, ankle = solve_leg(hip,ankle,'rear')
        directions = [(knee-hip).normalized(), (ankle-knee).normalized()]
        if previous:
            maximum = max(maximum, *[math.degrees(x.angle(y)) for x,y in zip(previous,directions)])
        previous = directions
    assert maximum < 45, maximum
    return maximum


OLD_KICK_MAX_STEP = check_old_kick()


def hinge_rotation(rest_delta, delta, turn):
    def basis(direction, lateral):
        y=direction.normalized()
        x=(lateral-y*lateral.dot(y)).normalized()
        return Matrix((x,y,x.cross(y))).transposed()
    lateral=Vector((0,1,0))
    return (basis(delta,Quaternion((0,0,1),turn)@lateral)@basis(rest_delta,lateral).transposed()).to_quaternion()


def pose(clip, frame):
    q = amount(frame,CLIPS[clip][2]) if CLIPS[clip][2] else 0
    jab = max(0,q) if clip == 'jab' else 0
    hook = q if clip == 'follow' else 0
    hammer = max(0,q) if clip == 'heavy' else 0
    lift = amount(frame,[(0,0),(9,1),(12,1),(18,0),(40,0)]) if clip == 'heavy' else 0
    kick = max(0,q) if clip == 'target' else 0
    spin = amount(frame,[(0,0),(4,.35),(9,1.45),(14,2.65),(17,2.65),(27,5.0),(35,2*math.pi),(40,2*math.pi)]) if clip == 'target' else 0
    if clip == 'target':
        hook = amount(frame,[(0,1),(2,1),(8,0),(40,0)])
    back, knee_drive = 0, 0
    if clip == 'multi':
        hook = amount(frame,[(0,0),(5,-.1),(8,1),(9,1),(16,0),(40,0)])
        back = amount(frame,[(0,0),(15,0),(18,-.1),(22,1),(23,1),(34,0),(40,0)])
    if clip == 'finisher':
        hook = amount(frame,[(0,0),(6,-.12),(12,1),(14,1),(20,0),(64,0)])
        knee_drive = amount(frame,[(0,0),(16,0),(21,.3),(28,1),(30,1),(36,0),(64,0)])
        hammer = amount(frame,[(0,0),(32,0),(43,1),(46,1),(56,.05),(64,0)])
        lift = amount(frame,[(0,0),(32,0),(37,1),(39,1),(43,0),(64,0)])
    palm = max(0,q) if clip in {'special','charged'} else 0
    gather = max(0,-q)*(5 if clip == 'special' else 2.5) if clip in {'special','charged'} else 0
    rise = max(0,q) if clip == 'low_heavy' else 0
    toe = max(0,q) if clip == 'low_light' else 0
    lowkick = max(0,q) if clip == 'low_medium' else 0
    crouching = clip in {'crouch','low_light','low_medium','low_heavy','blockstun_low'}
    drop = .42*(1-rise) if crouching else 0
    walk = clip in {'walk','walkback'}
    cycle = frame/24 if walk else 0
    breath = (1-math.cos(frame*2*math.pi/40))*.008 if clip == 'idle' else 0
    # Legacy bone names: lead is camera-near/right; rear is far/left.
    # Ryu's left foot/hand lead. His MP is planted, not our previous stepping hook.
    hip = Vector((.06*max(0,hook)+.02*jab+.22*palm+.12*hammer+.25*knee_drive,0,
                  .89-breath-drop-.28*lowkick-.08*palm-.035*max(0,hook)))
    hip_yaw = -.28-.10*jab-.14*hook+.18*palm+.95*hammer+1.55*rise+spin
    chest_yaw = -.42-.18*jab-.30*hook+.36*palm+1.45*hammer+1.85*rise+spin
    lean = .04+.045*jab+.05*max(0,hook)+.36*palm+.65*hammer-.90*lowkick-1.15*kick
    hip.x += .40*kick-.05*lowkick
    if clip == 'heavy':
        hop = amount(frame,[(0,0),(6,0),(11,1),(18,0),(23,0),(40,0)])
        hip.z += .12*hop
    else:
        hop = 0
    if walk:
        hip.z -= .026*(1-math.cos(cycle*4*math.pi))
        chest_yaw += .07*math.sin(cycle*2*math.pi)
    if clip == 'hitstun':
        hip.x -= q*.14; hip.z -= q*.13; chest_yaw += q*.55; lean = -q*.7
    if clip in {'blockstun','blockstun_low'}:
        hip.z -= q*.04 if not crouching else 0
        lean = -q*.07
    if clip == 'reload':
        hip.z -= .045*q; lean -= .14*q
    points = {'root': (Vector((0,0,0)),Vector((0,0,.12)))}
    rotations = {}
    p = hip
    for i,name in enumerate(['pelvis','abdomen.low','abdomen.high','chest','neck','head']):
        yaw = hip_yaw+(chest_yaw-hip_yaw)*min(i/3,1)
        tilt = lean*[.10,.70,.90,1,.20,.10][i]
        if i >= 4:
            yaw = -.12 # eyes stay on the opponent while the shoulders turn
            tilt += .15*palm
        rotation = Quaternion((0,1,0),tilt) @ Quaternion((0,0,1),yaw)
        tail = p+rotation@Vector((0,0,length(name)))
        points[name],rotations[name] = (p,tail),rotation
        p = tail
    for side,s in [('lead',-1),('rear',1)]:
        near = side == 'lead'
        shoulder = points['chest'][1]+rotations['chest']@Vector((0,s*.255,-.07))
        wrist = Vector((.18,-.22,1.64-drop) if near else (.36,.16,1.47-drop))
        hand_dir = Vector((.045,0,.13))
        pole = shoulder+Vector((-.18,s*.36,-.65))
        if not near:
            wrist = wrist.lerp(Vector((.97,.10,1.61)),jab)
            hand_dir = hand_dir.lerp(Vector((.14,0,0)),jab)
            if hook:
                wrist = wrist.lerp(Vector((.61,.14,1.30)) if hook>0 else Vector((.31,.19,1.57)),max(0,hook) if hook>0 else min(1,-hook*3))
                hand_dir = hand_dir.lerp(Vector((.13,0,-.045)),max(0,hook))
                pole = pole.lerp(shoulder+Vector((.28,.38,-.42)),max(0,hook))
        else:
            # Off hand stays at the cheek for both straight punches.
            wrist = wrist.lerp(Vector((.16,-.20,1.72)),max(jab,max(0,hook)))
        if back and near:
            wrist = wrist.lerp(Vector((.90,-.10,1.61)),max(0,back))
            hand_dir = hand_dir.lerp(Vector((.14,0,0)),max(0,back))
        if clip == 'heavy':
            if near:
                wrist = wrist.lerp(Vector((.04,-.15,2.05)),lift)
                wrist = wrist.lerp(Vector((.82,-.02,1.04)),hammer)
                pole = pole.lerp(shoulder+Vector((.48,-.25,.02)),max(lift,hammer))
                hand_dir = hand_dir.lerp(Vector((.04,0,-.14)),hammer)
            else:
                wrist = wrist.lerp(Vector((.02,.20,1.75)),max(lift,hammer))
        elif clip == 'finisher':
            wrist = wrist.lerp(Vector((.02,s*.09,2.10)),lift)
            wrist = wrist.lerp(Vector((.83,s*.07,1.28)),max(0,hammer))
            hand_dir = hand_dir.lerp(Vector((.10,0,-.10)),max(0,hammer))
        if rise:
            wrist = wrist.lerp(Vector((.60,-.06,1.68)) if near else Vector((.29,.12,1.41)),rise)
            if near:
                hand_dir = Vector((.025,0,.14))
                pole = shoulder+Vector((.36,-.33,-.55))
        if lowkick:
            wrist = wrist.lerp(Vector((-.55,-.27,.075)) if near else Vector((.10,.14,.85)),lowkick)
            if near:
                hand_dir = hand_dir.lerp(Vector((-.14,0,0)),lowkick)
                pole = pole.lerp(shoulder+Vector((-.38,-.14,-.12)),lowkick)
        if toe and not near:
            wrist = wrist.lerp(Vector((.20,.18,.68)),toe)
            hand_dir = hand_dir.lerp(Vector((.14,0,0)),toe)
        if palm or gather:
            # Hashogeki: dominant near/right palm, far hand tucked behind/below it.
            wrist = wrist.lerp(Vector((-.13,s*.23,1.22)),gather)
            wrist = wrist.lerp(Vector((.86,-.08,1.41)) if near else Vector((.60,.045,1.26)),palm)
            hand_dir = hand_dir.lerp(Vector((.02,0,.14)) if near else Vector((.04,-.015,.13)),palm)
        if kick:
            wrist = wrist.lerp(Vector((.12,-.15,1.74)) if near else Vector((-.10,.35,1.35)),kick)
            pole = pole.lerp(shoulder+Vector((-.55,-.30,.12) if near else (-.30,.48,-.10)),kick)
        if knee_drive:wrist = wrist.lerp(Vector((.38,s*.16,1.63)),knee_drive)
        if clip == 'reload':
            wrist = wrist.lerp(Vector((.08,s*.30,1.10)),q)
            hand_dir = hand_dir.lerp(Vector((.08,0,.07)),q)
        if clip == 'hitstun':wrist = wrist.lerp(Vector((-.10,-.45,1.25) if near else (-.30,.42,1.60)),q)
        if clip in {'blockstun','blockstun_low'}:wrist = wrist.lerp(Vector((.27,s*.14,1.76-drop)),q)
        if clip in {'jump','fall'}:wrist = wrist.lerp(Vector((.19,s*.29,1.60)),q)
        elbow,wrist = ik(shoulder,wrist,pole,length('upper.'+side),length('fore.'+side))
        points['upper.'+side],points['fore.'+side] = (shoulder,elbow),(elbow,wrist)
        points['hand.'+side] = (wrist,wrist+hand_dir.normalized()*length('hand.'+side))
        h = hip+rotations['pelvis']@Vector((0,s*.12,0))
        ankle = Vector(((-.12 if crouching else -.34) if near else .34,s*.17,.10))
        foot_dir = Vector((.18,0,0)); planted = True
        if palm:ankle.x += (.38 if not near else -.06)*palm
        if clip == 'heavy':
            ankle += Vector((.10*hammer,0,.08*hop))
            if not near:ankle += Vector((.12*hop,0,.25*hop))
            planted = False
        if clip == 'target':
            if near:
                radius = amount(frame,[(0,math.hypot(.34,.17)),(5,.30),(14,1.0),(17,1.0),(25,.64),(34,math.hypot(.34,.17)),(40,math.hypot(.34,.17))])
                angle = spin+math.atan2(-.17,-.34)
                height = amount(frame,[(0,.10),(5,.42),(14,1.67),(17,1.67),(25,1.28),(34,.10),(40,.10)])
                ankle = Vector((radius*math.cos(angle),radius*math.sin(angle),height))
                tilt = amount(frame,[(0,0),(8,.7),(14,1.0),(17,1),(29,.1),(34,0),(40,0)])
                heading = amount(frame,[(0,0),(5,-1.4),(14,0),(17,0),(27,1.6),(37,2*math.pi),(40,2*math.pi)])
                foot_dir = Vector((.18*math.cos(heading),.18*math.sin(heading),.25*tilt)).normalized()*.18
                planted = False
            else:
                foot_dir = Quaternion((0,0,1),spin)@foot_dir
        if rise and near:
            ankle.z += .13*rise
            foot_dir = Vector((.18,0,-.12*rise))
        if not near and toe:
            ankle = ankle.lerp(Vector((1.03,.17,.13)),toe); planted = False
        if near and lowkick:
            ankle = ankle.lerp(Vector((.96,-.17,.23)),lowkick)
            foot_dir = foot_dir.lerp(Vector((.15,0,.10)),lowkick).normalized()*.18; planted = False
        if lowkick and not near:ankle = ankle.lerp(Vector((.12,.17,.10)),lowkick)
        if clip == 'hitstun':
            ankle.x += (.08 if near else -.13)*q
            if near:ankle.z += .09*q
        if walk:
            t = ((-cycle if clip == 'walkback' else cycle)+(0 if near else .5))%1
            ankle.x = .29-.58*t*2 if t<.5 else -.29+.58*(t-.5)*2
            ankle.z += max(0,math.sin((t-.5)*2*math.pi))*(.08 if clip=='walkback' else .13); planted = False
        if clip in {'jump','fall'}:
            ankle = ankle.lerp(Vector((.28,s*.17,.37) if near else (.06,s*.17,.49)),q); planted = False
        if near and knee_drive:
            ankle = ankle.lerp(Vector((.60,-.18,.58)),knee_drive)
            foot_dir = foot_dir.lerp(Vector((.12,0,-.13)),knee_drive).normalized()*.18; planted = False
        knee,solved = solve_leg(h,ankle,side,spin)
        if planted:assert (solved-ankle).length<.000001,(clip,frame,side,'Planted foot out of reach')
        ankle = solved
        points['thigh.'+side],points['shin.'+side] = (h,knee),(knee,ankle)
        points['foot.'+side] = (ankle,ankle+foot_dir.normalized()*length('foot.'+side))
    for name,(p,tail) in points.items():
        delta = tail-p
        assert abs(delta.length-length(name))<.00001,(clip,frame,name)
        if name in rotations:rotation=rotations[name]
        else:
            rest_delta=Vector(REST[name][1])-Vector(REST[name][0])
            if name.startswith('foot.'):
                rotation=Quaternion((0,0,1),math.atan2(delta.y,delta.x))@Quaternion((0,1,0),-math.atan2(delta.z,math.hypot(delta.x,delta.y)))
            elif name.startswith(('thigh.','shin.')):
                rotation=hinge_rotation(rest_delta,delta,spin)
            else:rotation=rest_delta.rotation_difference(delta)
        rest_orientation=rig.data.bones[name].matrix_local.to_quaternion()
        matrix=Matrix.Translation(p)@(rotation@rest_orientation).to_matrix().to_4x4()
        assert all(math.isfinite(v) for row in matrix for v in row)
        rig.pose.bones[name].matrix=matrix
        bpy.context.view_layer.update()
    return points

pivot = world_to_camera_view(scene,cam,Vector((0,0,0)))
manifest = {'cell':[448,448], 'pivot':[pivot.x*448,(1-pivot.y)*448], 'fps':24, 'clips':{},
            'native_alpha':True, 'mesh_count':len(meshes), 'bone_count':len(REST), 'imported_assets':[],
            'old_kick_max_axis_step_degrees':OLD_KICK_MAX_STEP, 'reference_notes':'../README.md', 'human_verdict':'PROTOTYPE_LOOK_ACCEPTED; NEW_MOTION_PENDING', 'game_integrated':False}
rig.animation_data_create()
for name,(count,contact,keys) in CLIPS.items():
    action = bpy.data.actions.new(name)
    action.use_fake_user = True
    action['phase_notes'] = 'Target enters from the medium contact pose; lows enter/exit crouch; timing is presentation only.'
    rig.animation_data.action = action
    witnesses = {}
    previous = {}
    impact_bounds = []
    for f in range(count):
        scene.frame_set(f+1)
        pose(name,f)
        for bone in rig.pose.bones:
            # Choose quaternion hemisphere without make_compatible's renormalization
            # drift; identical loop poses must remain pixel-identical after baking.
            if bone.name in previous and bone.rotation_quaternion.dot(previous[bone.name]) < 0:
                bone.rotation_quaternion.negate()
            previous[bone.name] = bone.rotation_quaternion.copy()
            for prop in ['location','rotation_quaternion','scale']:
                bone.keyframe_insert(prop,frame=f+1,group=bone.name)
        if f+1 in {1,contact,count,*[v for pair in CONTACTS.get(name,[]) for v in pair]}:
            witnesses[str(f+1)] = {b.name:[list(row) for row in b.matrix] for b in rig.pose.bones}
        for hit,(start,end) in enumerate(CONTACTS.get(name,[])):
            if f+1 == start:
                obj = bpy.data.objects[STRIKES[name][hit]].evaluated_get(bpy.context.evaluated_depsgraph_get())
                evaluated = obj.to_mesh()
                verts = [world_to_camera_view(scene,cam,obj.matrix_world@v.co) for v in evaluated.vertices]
                impact_bounds.append([min(v.x for v in verts)*448,(1-max(v.y for v in verts))*448,
                                      max(v.x for v in verts)*448,(1-min(v.y for v in verts))*448])
                obj.to_mesh_clear()
    if name in CLOSED:
        for bone in witnesses['1']:
            assert max(abs(witnesses['1'][bone][r][c]-witnesses[str(count)][bone][r][c]) for r in range(4) for c in range(4)) < .00001
    manifest['clips'][name] = {'count':count, 'contact_frame':contact, 'contacts':CONTACTS.get(name,[]),
                              'impact_bounds':impact_bounds, 'witnesses':witnesses}

entry = manifest['clips']['follow']['witnesses']['13']
start = manifest['clips']['target']['witnesses']['1']
assert max(abs(entry[b][r][c]-start[b][r][c]) for b in entry for r in range(4) for c in range(4)) < .00001
manifest['clips']['target']['entry_from'] = {'clip':'follow','frame':13}
rig.animation_data.action = bpy.data.actions['idle']
rig.animation_data.action_slot = bpy.data.actions['idle'].slots[0]
scene.frame_start, scene.frame_end = 1, CLIPS['idle'][0]
scene.frame_set(1)
bpy.ops.object.select_all(action='DESELECT')
rig.select_set(True)
bpy.context.view_layer.objects.active = rig
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type == 'VIEW_3D':
            area.spaces.active.region_3d.view_perspective = 'CAMERA'
            area.spaces.active.shading.color_type = 'MATERIAL'
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'block-fighter.blend'))
(OUT/'manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
for name,(count,contact,keys) in CLIPS.items():
    if a.clip and name not in a.clip:
        continue
    rig.animation_data.action = bpy.data.actions[name]
    rig.animation_data.action_slot = bpy.data.actions[name].slots[0]
    frames = range(1,count+1) if a.render else sorted({1,contact,count//4+1,*[start for start,end in CONTACTS.get(name,[])]})
    for f in frames:
        scene.frame_set(f)
        scene.render.filepath = str(OUT/f'{name}-{f:03d}.png')
        bpy.ops.render.render(write_still=True)
# Neutral construction plate uses rest matrices; the saved source stays in guard.
rig.animation_data.action = None
for bone in rig.pose.bones:
    bone.matrix_basis = Matrix.Identity(4)
bpy.context.view_layer.update()
scene.render.filepath = str(OUT/'neutral.png')
bpy.ops.render.render(write_still=True)
print('BLOCK_PROOF_OK '+json.dumps({'output':str(OUT), 'meshes':len(meshes), 'bones':len(REST), 'clips':list(CLIPS)}),flush=True)
