# Original block mannequin, SF6 Ryu motion study

The simple prototype mesh style is retained. The user rejected the timing-only pass because the animations did not match Ryu. This pass reconstructs recognizable Ryu gestures on the existing 20 meshes / 19 bones. No imported rig, mesh, texture or motion; no model or dependency install. Human resemblance/feel approval is still pending.

## References and move-to-motion contract

Construction remains inspired by watchmeanimate's [The Blocks gallery](https://watchmeanimate.gumroad.com/l/qahdo), [trailer](https://www.youtube.com/watch?v=hOw1sBqP5qE) and [four-block torso demonstration](https://www.youtube.com/watch?v=h8IX6h_EEzA). That construction reference is **not** the attack-motion reference.

Motion references are the actual SF6 Ryu move images on [SuperCombo](https://wiki.supercombo.gg/w/Street_Fighter_6/Ryu), obtained from its [archived page](https://web.archive.org/web/20260922200846id_/https://wiki.supercombo.gg/w/Street_Fighter_6/Ryu) when the live site blocked access. The High Double Strike section explicitly reuses its 5HK image. Also inspected sampled visual storyboards and the transcript of [Frame Assist's Ryu vs Ken animation analysis](https://www.youtube.com/watch?v=NdAD5MPvxvI); those sparse samples are not a complete animation trace. The source image URLs, actual archive redirects and hashes are retained in the local proof's `reference/images.jsonl`.

- `jab` / 5L / Ryu 5LP: far/left straight jab; near hand protects the face; quick return to the same guard.
- `follow` / 5M / Ryu 5MP: compact, bent-arm body punch, off-hand guard and planted feet. Not the retired swinging hook.
- `heavy` / 5H / Collarbone Breaker 6MP: raised **single** arm, small preparatory hop, forward torso fold and descending strike. Not a two-hand hammer.
- `low_light` / 2L / Ryu 2LK: crouched, near-straight low left-leg check; one hand stays high and the other low.
- `low_medium` / 2M / Ryu 2MK: deep lateral lean, outboard hand brace, folded support leg and long low right-leg extension. Not a sweeping circular kick.
- `low_heavy` / 2H / Ryu 2HP: rising, turned torso, bent right-arm uppercut and lifted rear heel; return to crouch.
- `target` / 5M→5H / High Double Strike's 5HK: high turning kick, raised head-framing arm, counterlean, support-foot pivot, turn-through recovery. Not a chamber–piston–re-chamber side kick. Its entry remains the MP contact pose because this demo deliberately remaps the route.
- `special` / 5S / light Hashogeki, and `charged` / Lab medium Hashogeki: gather at the hip, staggered forward drive, dominant palm ahead of the supporting hand, recover to guard. Two timing variants of the same motion family, not Denjin-boosted Hashogeki.
- `reload` / 2S / Denjin-inspired utility: standing, grounded charge with hands drawn down; the instant ammo refill is still demo policy.

The rig's legacy `lead` bone suffix means **near/right**, and `rear` means **far/left**; the new guard's forward foot is the latter. Keep stable bone/clip identifiers rather than breaking stored assets. These are hand-authored prototype reconstructions of visible poses/gestures, not extracted Capcom keyframes or a frame-perfect clone. The source frame timings remain in [TIMING.md](../TIMING.md).

## Rebuild

```sh
"C:/Program Files/Blender Foundation/Blender 5.1/blender.exe" --background --factory-startup --python-exit-code 1 --python demo-wasm/art/block_fighter.py -- --render
"C:/Program Files/Blender Foundation/Blender 5.1/blender.exe" --background demo-wasm/art/generated/block-fighter.blend --python-exit-code 1 --python demo-wasm/art/block_fighter.py -- --check
python demo-wasm/art/pack_sprites.py
python demo-wasm/build.py
```

Uses installed Blender and Pillow. `block_fighter.py` is editable source; the generated `.blend` contains all named actions and the fixed orthographic camera. `generated/` is ignored local output. Verified RGBA atlases plus presentation metadata in `www/assets/blocks/` are the browser assets; ordinary builds need no Blender.

## Finite binding and verification

- Same four L/M/H/S buttons, seven lessons, 21 clips, camera and audio. Custom Twin Pulse and the three-contact finisher are explicitly demo extensions, not extra Ryu moves.
- Simulation still owns timing, hitstop, contacts, cancels, damage and resources. Presentation maps each source contact hold to its authored native hit window, including gaps between multi-hit contacts.
- MP and the target kick no longer translate the actor forward by six units during startup: the reference uses planted weight transfer/pivots. Small host pushback (MP 1, target 2) retains spacing feedback and the existing routes. The native movement feature remains supported and tested with an authored travel example.
- The descending overhead and high target kick's hitboxes follow their changed impact heights. These are demo collision boxes, **not copied SF6 hitboxes**. Arbitrary editor changes do not procedurally retarget the mesh.
- `--check` reopens every saved action, checking matrix witnesses, every rigid mesh edge, floor clearance and per-frame leg rotation. It also checks the nine attacks' identifying limb/height/brace relationships and standing charge. The retired art fails the new limb-identity check.
- A transported leg hinge plane prevents the high turn's bend-plane flip; a stable foot heading prevents roll flips. Sign-only quaternion hemisphere selection avoids baking roundoff into otherwise identical loop endpoints. The original knee-flip trajectory remains a runnable regression; limits were not relaxed.
- `pack_sprites.py [render-directory]` validates the exact roster/frame count, transparent crop bounds, visible loop endpoints, exact MP→target entry and **every** atlas cell after PNG readback.
- Existing native/browser tests cover real input, all routes, mirrored contacts, block/whiff, hitstop, replay, rejected input, editor/export and CLI binary identity. Direct 1× gameplay and reference comparisons go to the user; machine checks are not a human verdict.

For a bounded future repair, `--render --clip follow` (repeat `--clip` as needed) can reuse an otherwise complete render directory. A guard/stance or shared IK change affects every clip and requires a complete render. No generated Blender file, full-resolution sequence or third-party reference media is committed by default. Review uses direct attachments or Taste Gate, not Continuity Studio.
