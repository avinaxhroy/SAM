/**
 * THE ROOM'S LIGHT, WHILE A TURN RUNS (§4.7 — UI_SPEC · The working veil).
 *
 * The veil was two CSS sweeps of the assistant's own hues carried across the
 * window on `background-position`. It behaved — it was cheap, it needed no GPU,
 * and reduced motion was one `animation: none` — but it read as what it was:
 * two soft bands sliding on a loop. The room should look *lit*, by something
 * that is moving, over something that is not.
 *
 * That is why this is only half the veil. The other half is `material.ts`: a
 * grain painted once into a canvas underneath, which never moves. This file is
 * the light travelling across it, and the division is the whole idea — a
 * texture that moves with the light is a pattern being carried, not a surface
 * being lit. The shader's own tooth is therefore sampled in *screen* space
 * (`gl_FragCoord`), so what it adds lands on the material rather than on the
 * flow.
 *
 * The field is built from:
 *
 *   · **five incommensurate sine sheets.** No lattice, so no direction for the
 *     eye to find; no common period, so no loop to notice.
 *   · **a warp of a warp.** The sheets are looked up through themselves twice,
 *     so the value that carries the light arrives along curved paths. This is
 *     the difference between fluid and band: a band is a function of one axis
 *     and a clock, a warp is a function of the room.
 *   · **one light axis, in window units.** The room is lit from one side, and
 *     the position that decides where is mixed with the flow's own value.
 *     Without the axis every crest is a blob and the window reads as mottled
 *     rather than as lit.
 *   · **a gate, not a floor.** `band` and `core` decide where light exists at
 *     all, so between ribbons the room is the scrim and nothing else. A floor
 *     of any size across a full window is haze, and haze is what this replaced.
 *
 * Colour is not invented here. The ramp is read from the design tokens
 * (`--ai-field-a/b/c`) as *resolved* values: a probe element wears `var(--…)`
 * and a 1×1 canvas reads the pixel back, so `color-mix`, `oklch` and whatever a
 * theme does to those tokens stay the theme layer's business — this file never
 * spells a colour, which is also what keeps it inside D17. The read repeats
 * whenever the document element's attributes change, which is how a light/dark
 * switch, or a user theme, reaches a shader that is already running.
 *
 * Three rules, each load-bearing:
 *
 * 1 · **Nothing here is required.** No WebGL2, a link failure, a token that
 *     will not resolve to pixels, a lost context — each says so and hands the
 *     layer back to the CSS sweeps it shipped with. The scrim, the grain and
 *     the pointer set belong to `.cd-ai-veil`; this only draws over them.
 * 2 · **It costs what a background costs.** Capped at 1.5 device pixels per CSS
 *     pixel and thirty frames a second, re-reads nothing per frame, and when
 *     the OS asks for reduced motion it draws *one still frame of the same
 *     field* — the state stays legible, it just stops moving (§11).
 * 3 · **The clock only runs forward.** Flow time accumulates from real, clamped
 *     deltas and is never taken modulo anything, so there is no loop to notice
 *     and an hour-long turn does not repeat itself.
 */

export type FieldState = 'live' | 'fallback';

export type VeilField = {
  /** `fallback` means the layer's own CSS sweeps are the veil; nothing is drawn. */
  readonly state: FieldState;
  destroy(): void;
};

/** Past this the shader is fill-bound and the extra pixels buy nothing. */
export const MAX_DPR = 1.5;
/** Thirty, not sixty: the flow is slow, and a still room should stay cool. */
const FRAME_MS = 1000 / 30;
/** Where the still frame sits — far enough in that the field has a shape. */
const STILL_SECONDS = 11;

const TOKENS = {
  lo: '--ai-field-a',
  mid: '--ai-field-b',
  hi: '--ai-field-c',
} as const;

const SCALARS = {
  scale: ['--ai-field-scale', 2.2],
  light: ['--ai-field-light', 1.0],
  chroma: ['--ai-field-chroma', 2.2],
  rate: ['--ai-field-rate', 0.1],
  texture: ['--ai-texture', 0.32],
} as const;

const VERT = `#version 300 es
void main() {
  vec2 corner = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}`;

const FRAG = `#version 300 es
precision highp float;

uniform vec2  uRes;    // drawing buffer, device px
uniform float uT;      // seconds of flow
uniform vec3  uLo;     // the ramp, trough to crest
uniform vec3  uMid;
uniform vec3  uHi;
uniform float uScale;  // eddies per height of the room
uniform float uLight;
uniform float uChroma;
uniform sampler2D uMat;   // LAYER 1 — the material, a 256px tile that repeats
uniform float uMatPx;     // its edge in device pixels
uniform float uTexture;   // how hard the light shows the surface it falls on

out vec4 outColor;

/* One pixel of noise, for the material only: the flow itself is never sampled
   through this, or the grain would travel with the light. */
float hash(vec2 p) {
  vec3 q = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
  q += dot(q, q.yzx + 33.33);
  return fract((q.x + q.y) * q.z);
}

/* The flow, as a sum of travelling waves on incommensurate directions.
   Deliberately not a value-noise lattice: an fbm's cells are flat plates at the
   scale it is sampled at, and over a whole window those plates read as squares,
   not as light. Waves have no lattice at all, they are C-infinity, and their
   derivatives — which the filaments below are read off — are exact. */
float flow(vec2 p, float t) {
  float v = 0.0;
  v += sin(dot(p, vec2( 1.00,  0.55)) * 1.31 + t * 0.51);
  v += sin(dot(p, vec2(-0.62,  1.00)) * 1.73 - t * 0.37);
  v += sin(dot(p, vec2( 0.83, -1.00)) * 2.29 + t * 0.29);
  v += sin(dot(p, vec2( 1.00,  1.21)) * 3.07 - t * 0.23);
  v += sin(dot(p, vec2(-1.00,  0.37)) * 4.13 + t * 0.17);
  return v * 0.2;
}

void main() {
  vec2 p = gl_FragCoord.xy / uRes.y;
  p.x -= 0.5 * uRes.x / uRes.y;
  float t = uT;

  /* Stretched across the room's width, and drifting up it. A field sampled
     square is isotropic, and isotropic light reads as fog however it moves;
     light in a room travels along the width, in ribbons. */
  vec2 st = vec2(0.42, 1.30) * uScale;
  vec2 sheet = p * st + vec2(0.0, -t * 0.16);

  /* The warp, then the warp of the warp — the difference between a wave and a
     fluid is that the second one is looked up through the first. */
  vec2 a = vec2(
    flow(p * vec2(0.30, 0.85) + vec2(0.0, 1.7), t * 0.55),
    flow(p * vec2(0.30, 0.85) + vec2(4.3, 2.9), t * 0.55)
  );
  vec2 b = vec2(
    flow(p * vec2(0.55, 1.45) + 1.6 * a + vec2(2.7, 0.4), t * 0.80),
    flow(p * vec2(0.55, 1.45) + 1.6 * a + vec2(6.1, 5.2), t * 0.80)
  );
  vec2 q = sheet + 2.1 * b + 0.9 * a;

  /* Two sheets: one scale of light is a gradient, two is a depth. */
  float f = flow(q, t * 0.90) * 0.74 + flow(q * 2.3 + 5.0, t * 1.60) * 0.26;

  /* The light has an axis, and the field's job is to bend it rather than to
     invent it. A window lit by a gradient has a direction the eye can name —
     it falls the way the app's own shadows fall, from up and to the right; a
     window lit only by a scalar field has no direction at all, and no
     direction is the whole reason a field of this kind reads as fog. */
  vec2 axis = normalize(vec2(0.86, 0.50));
  /* Window units, not field units: the axis is a property of the room's light
     and it must not shrink when the fluid's own scale does. */
  float across = dot(p, axis) * 0.25;
  float v = clamp(0.5 + across * 0.55 + 0.42 * f, 0.0, 1.0);

  /* The gathering *is* the shape. Below the gate the room is only scrimmed and
     the field is not there at all; above it the light arrives, first as the
     violet the room is lit with, then as the rose it turns where it is
     thickest, then as the white a light goes where there is nothing left to
     tint. Hue and coverage are driven by the same number, so a ribbon carries a
     gradient across its own width and has no edge anywhere to give it away. */
  float band = smoothstep(0.50, 0.78, v);
  float core = smoothstep(0.58, 0.88, v);
  vec3 tint = mix(uLo, uMid, core);
  tint = mix(tint, uHi, core * core);

  /* Modest tokens, real colour: the ramp is pulled away from its own luma so
     the room reads as lit rather than as washed out, without asking the token
     register for chroma it does not own. */
  float luma = dot(tint, vec3(0.2126, 0.7152, 0.0722));
  tint = max(mix(vec3(luma), tint, uChroma), vec3(0.0));

  float cover = 0.72 * pow(band, 1.15);
  cover *= 0.94 + 0.06 * sin(t * 0.37 + v * 5.0);

  vec3 light = tint * (cover * uLight);
  light = mix(light, vec3(1.0), core * 0.30);

  /* Layer 1 does the last thing, and it is the last thing on purpose: the
     light is *multiplied* by the surface it lands on, so a fibre can only be
     seen where there is light to see it by. Sampled in screen space from the
     painted tile, so the surface is still while the light crosses it — and
     sampling in the same space the room is drawn in is what keeps a 256px tile
     from sliding, smearing or swimming under the flow.
     'r' is the polarity and 'a' the amplitude, so one channel carries a signed
     tooth: white lifts the light, black takes it away, and between them the
     surface is neutral. No blend mode anywhere in the chain. */
  vec2 muv = gl_FragCoord.xy / uMatPx;
  vec4 mat = texture(uMat, muv);
  float tooth = mat.r * 2.0 - 1.0;
  /* Asymmetric on purpose: a raised thread hides the light behind it, and only
     the top of one catches it. Weighting the two sides unequally is what keeps
     this a surface instead of a sparkle — the bright side never gets far enough
     to clip to white, and clipping is what read as salt. */
  tooth *= mat.a * (tooth < 0.0 ? 1.0 : 0.45);
  light *= 1.0 + tooth * uTexture;

  light = max(light, vec3(0.0));
  /* Premultiplied, with the light itself as the coverage: where the room is
     not lit the scrim and the grain below show through untouched. */
  float alpha = clamp(max(max(light.r, light.g), light.b), 0.0, 1.0);
  outColor = vec4(clamp(light, vec3(0.0), vec3(1.0)), alpha);
}`;

type Rgb = [number, number, number];

type Reader = ((token: string) => Rgb | null) & { release(): void };

type Settings = {
  lo: Rgb;
  mid: Rgb;
  hi: Rgb;
  scale: number;
  light: number;
  chroma: number;
  rate: number;
  texture: number;
};

/**
 * Turns a token into bytes, without ever writing a colour in this file: a
 * probe wears `var(--token)`, the browser resolves it (`color-mix`, `oklch`,
 * a user theme — all the same to us), and a 1×1 canvas is the thing that knows
 * how to make a number out of whatever it serialised to. A refused assignment
 * leaves the pixel transparent, which is the one signal worth trusting.
 */
function reader(): Reader {
  const probe = document.createElement('span');
  probe.setAttribute('aria-hidden', 'true');
  probe.style.cssText =
    'position:fixed;left:-10px;top:-10px;width:1px;height:1px;visibility:hidden;pointer-events:none';
  document.body.append(probe);

  const pad = document.createElement('canvas');
  pad.width = 1;
  pad.height = 1;
  const ctx = pad.getContext('2d', { willReadFrequently: true });

  const read = ((token: string) => {
    if (!ctx) return null;
    probe.style.color = `var(${token})`;
    const resolved = getComputedStyle(probe).color;
    if (!resolved) return null;
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = resolved;
    ctx.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
    // A refused assignment leaves the cleared pixel behind; that is the one
    // signal worth trusting, and it means "fall back", not "guess".
    if (!a) return null;
    // Straight, not premultiplied, whatever alpha the token resolved to.
    const k = 255 / a;
    return [
      Math.min(1, (r * k) / 255),
      Math.min(1, (g * k) / 255),
      Math.min(1, (b * k) / 255),
    ] as Rgb;
  }) as Reader;

  read.release = () => probe.remove();
  return read;
}

function shader(gl: WebGL2RenderingContext, kind: number, source: string) {
  const made = gl.createShader(kind);
  if (!made) return null;
  gl.shaderSource(made, source);
  gl.compileShader(made);
  if (gl.getShaderParameter(made, gl.COMPILE_STATUS)) return made;
  gl.deleteShader(made);
  return null;
}

function link(gl: WebGL2RenderingContext) {
  const vs = shader(gl, gl.VERTEX_SHADER, VERT);
  const fs = shader(gl, gl.FRAGMENT_SHADER, FRAG);
  if (!vs || !fs) return null;
  const program = gl.createProgram();
  if (!program) return null;
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.linkProgram(program);
  gl.deleteShader(vs);
  gl.deleteShader(fs);
  if (gl.getProgramParameter(program, gl.LINK_STATUS)) return program;
  gl.deleteProgram(program);
  return null;
}

/**
 * Draws the field on `canvas` until the returned handle is destroyed. Never
 * throws: a return of `state: 'fallback'` is a normal outcome and means the
 * caller should let the layer's CSS be the veil.
 */
export function mountVeil(
  canvas: HTMLCanvasElement,
  material: HTMLCanvasElement | null,
  onstate: (state: FieldState) => void = () => {},
): VeilField {
  const gl = canvas.getContext('webgl2', {
    alpha: true,
    premultipliedAlpha: true,
    antialias: false,
    depth: false,
    stencil: false,
    powerPreference: 'low-power',
    // The still frame reduced motion gets is drawn once and must survive being
    // composited; without this the buffer may be dropped and the veil go blank.
    preserveDrawingBuffer: true,
  });

  const program = gl && link(gl);
  const read = reader();
  let settings = gl && program ? readSettings(gl, program, read) : null;

  if (!gl || !program || !settings) {
    return { state: 'fallback', destroy: () => {} };
  }

  const context = gl;
  const shaders = program;
  const uniforms = {
    res: context.getUniformLocation(shaders, 'uRes'),
    t: context.getUniformLocation(shaders, 'uT'),
    lo: context.getUniformLocation(shaders, 'uLo'),
    mid: context.getUniformLocation(shaders, 'uMid'),
    hi: context.getUniformLocation(shaders, 'uHi'),
    scale: context.getUniformLocation(shaders, 'uScale'),
    light: context.getUniformLocation(shaders, 'uLight'),
    chroma: context.getUniformLocation(shaders, 'uChroma'),
    texture: context.getUniformLocation(shaders, 'uTexture'),
    mat: context.getUniformLocation(shaders, 'uMat'),
    matPx: context.getUniformLocation(shaders, 'uMatPx'),
  };

  /* LAYER 1, as the one texture this shader reads: the painted tile, uploaded
     once and wrapped by the GPU. This is the only moment the material costs
     anything — after it, the surface is a sampler read, and re-uploading is
     never needed because the tile is deterministic and fixed. */
  const tile = material && material.width ? material : null;
  if (tile) {
    context.activeTexture(context.TEXTURE0);
    context.bindTexture(context.TEXTURE_2D, context.createTexture());
    context.pixelStorei(context.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    context.texImage2D(
      context.TEXTURE_2D,
      0,
      context.RGBA,
      context.RGBA,
      context.UNSIGNED_BYTE,
      tile,
    );
    context.texParameteri(context.TEXTURE_2D, context.TEXTURE_WRAP_S, context.REPEAT);
    context.texParameteri(context.TEXTURE_2D, context.TEXTURE_WRAP_T, context.REPEAT);
    context.texParameteri(context.TEXTURE_2D, context.TEXTURE_MIN_FILTER, context.LINEAR);
    context.texParameteri(context.TEXTURE_2D, context.TEXTURE_MAG_FILTER, context.LINEAR);
    context.uniform1i(uniforms.mat, 0);
  }

  // WebGL2 needs no attributes for the fullscreen triangle, but a bound vertex
  // array object is what the spec's default state expects; one empty one is
  // cheaper than the argument about whether the default is still valid.
  const vao = context.createVertexArray();
  context.bindVertexArray(vao);
  context.useProgram(shaders);

  let clock = 0;
  let frame = 0;
  let drew = 0;
  let last = 0;
  let dead = false;
  let pixel = 1;

  const motion = matchMedia('(prefers-reduced-motion: reduce)');

  /** The one place that touches layout: the observer's, never a frame's. */
  function measure() {
    const box = canvas.getBoundingClientRect();
    const dpr = Math.min(window.devicePixelRatio || 1, MAX_DPR);
    const width = Math.max(1, Math.round(box.width * dpr));
    const height = Math.max(1, Math.round(box.height * dpr));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    pixel = width / Math.max(box.width, 1);
  }

  function draw(still: boolean) {
    if (dead || !settings) return;
    const now = settings;
    context.viewport(0, 0, canvas.width, canvas.height);
    context.uniform2f(uniforms.res, canvas.width, canvas.height);
    context.uniform1f(uniforms.t, still ? STILL_SECONDS : clock);
    // The tile repeats every `tile.width` CSS px, whatever the device ratio:
    // `gl_FragCoord` is in device pixels, so the divisor has to be too.
    context.uniform1f(uniforms.matPx, tile ? tile.width * pixel : 256);
    context.uniform3f(uniforms.lo, now.lo[0], now.lo[1], now.lo[2]);
    context.uniform3f(uniforms.mid, now.mid[0], now.mid[1], now.mid[2]);
    context.uniform3f(uniforms.hi, now.hi[0], now.hi[1], now.hi[2]);
    context.uniform1f(uniforms.scale, now.scale);
    context.uniform1f(uniforms.light, now.light);
    context.uniform1f(uniforms.texture, now.texture);
    context.uniform1f(uniforms.chroma, now.chroma);
    context.drawArrays(context.TRIANGLES, 0, 3);
  }

  function step(now: number) {
    if (dead) return;
    frame = requestAnimationFrame(step);
    if (!last) last = now;
    const dt = Math.min((now - last) / 1000, 0.1);
    last = now;
    if (settings) clock += dt * settings.rate;
    if (now - drew < FRAME_MS) return;
    drew = now;
    draw(false);
  }

  function play() {
    cancelAnimationFrame(frame);
    if (dead) return;
    if (motion.matches) {
      draw(true);
      return;
    }
    last = 0;
    frame = requestAnimationFrame(step);
  }

  function resize() {
    measure();
    // A still field has to be redrawn at the new size; a running one gets it on
    // its next frame, which is a frame away.
    if (motion.matches) draw(true);
  }

  measure();

  const observer = new ResizeObserver(resize);
  observer.observe(canvas);

  const themed = new MutationObserver(() => {
    if (dead || !gl) return;
    const next = readSettings(context, shaders, read);
    if (next) settings = next;
    draw(motion.matches);
  });
  themed.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['style', 'data-theme', 'data-theme-id'],
  });

  const onMotion = () => play();
  motion.addEventListener('change', onMotion);

  // A background tab stops firing frames anyway; this keeps the clock honest
  // across the gap instead of spending it all on one delta.
  const onVisible = () => {
    last = 0;
  };
  document.addEventListener('visibilitychange', onVisible);

  const onLost = (event: Event) => {
    event.preventDefault();
    stop();
    onstate('fallback');
  };
  canvas.addEventListener('webglcontextlost', onLost);

  function stop() {
    if (dead) return;
    dead = true;
    cancelAnimationFrame(frame);
    frame = 0;
    observer.disconnect();
    themed.disconnect();
    motion.removeEventListener('change', onMotion);
    document.removeEventListener('visibilitychange', onVisible);
    canvas.removeEventListener('webglcontextlost', onLost);
  }

  play();

  return {
    state: 'live',
    destroy() {
      stop();
      read.release();
      context.deleteVertexArray(vao);
      context.deleteProgram(shaders);
      context.getExtension('WEBGL_lose_context')?.loseContext();
    },
  };
}

function readSettings(
  gl: WebGL2RenderingContext,
  program: WebGLProgram,
  read: ReturnType<typeof reader>,
): Settings | null {
  const lo = read(TOKENS.lo);
  const mid = read(TOKENS.mid);
  const hi = read(TOKENS.hi);
  if (!lo || !mid || !hi) return null;
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) return null;
  const root = getComputedStyle(document.documentElement);
  const scalar = ([name, fallback]: readonly [string, number]) => {
    const value = Number.parseFloat(root.getPropertyValue(name));
    return Number.isFinite(value) ? value : fallback;
  };
  return {
    lo,
    mid,
    hi,
    scale: scalar(SCALARS.scale),
    light: scalar(SCALARS.light),
    chroma: scalar(SCALARS.chroma),
    rate: scalar(SCALARS.rate),
    texture: scalar(SCALARS.texture),
  };
}
