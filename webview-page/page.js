// crest-synth projection page.
//
// PURE RENDER. `render(model)` rebuilds the five shell bands from one
// deserialized SemanticGraphicalViewModel document and nothing else: no
// Date.now, no Math.random, no accumulated state, no incidental
// iteration-order dependence (every walk follows the document's own array
// order). The same document always paints the same DOM. Both declared
// top-level contexts render here — MIXER as the sixteen-column strip bank,
// PATCH as the listed strip of projected rows — through the same shared
// structural bands (context line, identity header, workspace scaffold,
// persistent side region, footer); nothing forks the schema and no field is
// invented.
//
// The one carve-out is presentation-only meter animation:
// the crest://meters listener repaints ONLY the meter element from the
// latest AudioObservationSnapshot frame, mirroring the retired-shell rule — a
// reading shows only when the frame's parameterGeneration and
// activeGraphRevision both match the document on screen; a missing or stale
// frame reads the zero state.
//
// This page registers no key handler and captures no input; keys are
// captured Rust-side (WP01/WP02 boundary). Listener glue (tauri events →
// parse → render → paint ack) is kept separate from the pure function so the
// acceptance harness can drive `render`/`renderObservation` headlessly.
"use strict";

(function pageModule() {
  var PROJECTION_EVENT = "crest://projection";
  var METER_EVENT = "crest://meters";
  var PAINTED_EVENT = "crest://painted";
  var READY_EVENT = "crest://ready";
  var RENDER_ERROR_EVENT = "crest://render-error";

  // Explicit-unavailability mark: a declared structure with no view data
  // behind it says so; it is never painted with a representative value and
  // never dressed as the resting state (MixerTrackColumnStructure rule).
  var UNAVAILABLE = "UNAVAILABLE";

  // The row-level unavailable mark the pre-cutover shipped rows painted
  // (parameter_row::UNAVAILABLE_MARK): a designed row whose value data did
  // not arrive reads "--" where its value would be.
  var UNAVAILABLE_MARK = "--";

  // The authored hint separator (primitives::hint::HINT_SEPARATOR).
  var HINT_SEPARATOR = " · ";

  // The declared column anatomy, closed and ordered. Rendering walks exactly this
  // list; the observation reports it per column so a lost or reordered
  // structure is measurable.
  var COLUMN_ANATOMY = [
    "TrackHeader",
    "LevelFader",
    "LevelReadout",
    "PanReadout",
    "StateLine",
  ];

  // The five entries the PATCH Utility panel is designed to carry, in
  // authored order (utility_inspector_panel::DESIGNED_UTILITY_ENTRIES).
  // `driver: null` means the projection carries no identity for the entry at
  // all; it is marked explicitly unavailable rather than omitted or invented.
  // All five entries are now driven: the reducer carries a canonical value and
  // a focus identity for every row the design draws.
  var DESIGNED_UTILITY_ENTRIES = [
    { label: "MASTER VOLUME", driver: "patch.global.masterGainDb" },
    { label: "PATCH VOLUME", driver: "patch.output.trimGainDb" },
    { label: "MIDI INPUT", driver: "patch.midiInput" },
    { label: "OUTPUT TRACK", driver: "patch.output.outputTrack" },
    { label: "VOICE LIMIT", driver: "patch.voiceLimit" },
  ];

  // The PATCH strip's designed group anatomy, closed and ordered — the PATCH
  // twin of COLUMN_ANATOMY (crest-spec valueObject.Shell.ShellComposition,
  // `PatchStrip`: "arranges groups rather than controls ... a flat run erases
  // which rows belong to which slot").
  //
  // `legend` is the authored name of the designed structure, taken from the
  // design file (SCREEN · Patch Strip · 1920×1080: `Instrument` 36:28,
  // `AMP ENVELOPE / ADSR PREVIEW` 36:26, `FX 1`/`FX 2`/`FX 3` 36:34/39/44).
  // It is a static structure name like COLUMN_ANATOMY's entries and
  // DESIGNED_UTILITY_ENTRIES' labels, never a projected control label and
  // never a serialization key.
  //
  // `designed: true` means the design always draws this group, so a group the
  // projection supplies no row for is marked explicitly unavailable *inside
  // its own group* rather than vanishing. `designed: false` marks the one
  // descriptor-driven group: an instrument that declares no parameters
  // genuinely has none, which is a projected fact and not missing data.
  //
  // The order is the order the projection emits, and it has to be: the
  // capability's own rows belong to the instrument the engine row names, but
  // the projection emits them *after* the envelope (DESIGN.md: "Engine,
  // Attack, Decay, Sustain, Release, descriptor-declared instrument
  // StructuralChoice rows"), so a group that gathered them under the engine
  // row would paint them out of the reducer's own order. Grouping is a
  // presentation change; it does not get to move a row.
  var DESIGNED_STRIP_GROUPS = [
    { key: "instrument", legend: "INSTRUMENT", designed: true },
    { key: "envelope", legend: "AMP ENVELOPE", designed: true },
    { key: "capability", legend: null, designed: false },
    { key: "slot.0", legend: "SLOT 1", designed: true },
    { key: "slot.1", legend: "SLOT 2", designed: true },
    { key: "slot.2", legend: "SLOT 3", designed: true },
  ];

  // The authored range separator, the same one the Inspector's own range
  // reading already uses ("LEVEL 00 — 7F").
  var RANGE_SEPARATOR = " — ";

  // The non-color mark a capability-declared read-only row carries. The row
  // also takes a dashed keyline in page.css, so the declaration reads in text
  // and in shape, not in color alone (ComponentState rule).
  var READ_ONLY_MARK = "READ-ONLY";

  // ---- pure helpers ------------------------------------------------------

  function escapeHtml(text) {
    return String(text).replace(/[&<>"']/g, function (ch) {
      return {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      }[ch];
    });
  }

  // Unwraps the nested value shapes the document serializes:
  // {kind:"scalar", value:0.5} and {kind:"parameter", value:{kind:"toggle",
  // value:false}} both resolve to their innermost primitive.
  function innerValue(control) {
    var value = control && control.value;
    var guard = 0;
    while (value !== null && typeof value === "object" && guard < 8) {
      value = value.value;
      guard += 1;
    }
    return value;
  }

  function toggleOn(control) {
    var value = innerValue(control);
    return value === true || value === 1;
  }

  // Normalized position of a continuous control inside its declared range.
  function fraction(control) {
    var range = control && control.numericRange;
    var value = innerValue(control);
    if (!range || typeof value !== "number" || range.maximum === range.minimum) {
      return 0;
    }
    var f = (value - range.minimum) / (range.maximum - range.minimum);
    return Math.min(1, Math.max(0, f));
  }

  // Where a row's value sits inside its declared bounds, or null when the
  // document carries no bounds or no numeric value — a row drawn half-full
  // because nobody said how full it is would be a claim the projection never
  // made (parameter_row::position_fraction).
  function positionFraction(control) {
    var range = control && control.numericRange;
    var value = innerValue(control);
    if (!range || typeof value !== "number" || range.maximum === range.minimum) {
      return null;
    }
    return fraction(control);
  }

  // The bound MidiHexadecimal form: (v-min)/(max-min)*127 as two uppercase
  // hex digits. Bound to the LevelReadout presentation (and the Inspector's
  // reading of the same focused level).
  function midiHex(control) {
    var scaled = Math.round(fraction(control) * 127);
    var hex = scaled.toString(16).toUpperCase();
    return hex.length < 2 ? "0" + hex : hex;
  }

  function midiDecimal(control) {
    return String(Math.round(fraction(control) * 127));
  }

  // The authored pan label-and-condition pair; a centered pan is a presented
  // value ("C"), never an absence.
  function panCondition(control) {
    var f = fraction(control);
    if (f < 0.45) {
      return "L";
    }
    return f > 0.55 ? "R" : "C";
  }

  function trackName(trackId) {
    var hex = Number(trackId).toString(16).toUpperCase();
    return "T" + (hex.length < 2 ? "0" + hex : hex);
  }

  function surfaceById(model, id) {
    var surfaces = model.surfaces || [];
    for (var i = 0; i < surfaces.length; i += 1) {
      if (surfaces[i].id === id) {
        return surfaces[i];
      }
    }
    return null;
  }

  // The document's surface holding one declared role, resolved by that role
  // rather than by a context fork: `persistentSide` is the Inspector on MIXER
  // and the Utility region on PATCH, `detail` is the subordinate capability
  // surface present exactly while a detail entry is open.
  function surfaceByRole(model, role) {
    var surfaces = model.surfaces || [];
    for (var i = 0; i < surfaces.length; i += 1) {
      if (surfaces[i].role === role) {
        return surfaces[i];
      }
    }
    return null;
  }

  function persistentSideSurface(model) {
    return surfaceByRole(model, "persistentSide");
  }

  // The serialized identity of one projected control ("patch.engine",
  // "patch.effectSlot.0", ...). Used for structure — which group a row joins,
  // which row heads it — and never painted; the same way the mixer bank
  // groups its columns by `id.trackId`.
  function controlIdOf(control) {
    var id =
      control && control.path && control.path.controlId
        ? control.path.controlId.id
        : "";
    return id !== null && typeof id === "object"
      ? JSON.stringify(id)
      : String(id);
  }

  // One surface's control by identity, or null.
  function controlById(surface, id) {
    var controls = (surface && surface.controls) || [];
    for (var i = 0; i < controls.length; i += 1) {
      if (controlIdOf(controls[i]) === id) {
        return controls[i];
      }
    }
    return null;
  }

  function startsWith(text, prefix) {
    return text.lastIndexOf(prefix, 0) === 0;
  }

  // Groups the flat mixer-main control list into track columns keyed by
  // track id, preserving the document's declared first-appearance order.
  function trackColumns(mainSurface) {
    var order = [];
    var byTrack = {};
    var controls = (mainSurface && mainSurface.controls) || [];
    for (var i = 0; i < controls.length; i += 1) {
      var control = controls[i];
      var id = control.path.controlId.id;
      if (!id || id.kind !== "track") {
        continue;
      }
      if (!Object.prototype.hasOwnProperty.call(byTrack, id.trackId)) {
        byTrack[id.trackId] = { trackId: id.trackId };
        order.push(byTrack[id.trackId]);
      }
      byTrack[id.trackId][id.parameter] = control;
    }
    return order;
  }

  function columnFocused(column) {
    var parameters = ["level", "pan", "mute", "solo"];
    for (var i = 0; i < parameters.length; i += 1) {
      var control = column[parameters[i]];
      if (control && control.focused) {
        return true;
      }
    }
    return false;
  }

  function columnFocusedRow(column) {
    var parameters = ["level", "pan", "mute", "solo"];
    for (var i = 0; i < parameters.length; i += 1) {
      var parameter = parameters[i];
      if (column[parameter] && column[parameter].focused) {
        return parameter;
      }
    }
    return null;
  }

  function mixerRowState(control, mode) {
    return control ? controlState(control, mode).name : "unavailable";
  }

  // Fill/emphasis state for the column's fader, resolved in declared
  // priority: focus, then error, then disabled, then mute, then solo, then
  // rest. Each is also carried by text or shape elsewhere in the column, so
  // color is never the only signal.
  function faderState(column) {
    if (columnFocused(column)) {
      return "focused";
    }
    var level = column.level;
    if (level && level.error) {
      return "error";
    }
    if (level && level.enabled === false) {
      return "disabled";
    }
    if (column.mute && toggleOn(column.mute)) {
      return "muted";
    }
    if (column.solo && toggleOn(column.solo)) {
      return "soloed";
    }
    return "resting";
  }

  // The focused control anywhere in the document, in surface order.
  function focusedControl(model) {
    var surfaces = model.surfaces || [];
    for (var s = 0; s < surfaces.length; s += 1) {
      var controls = surfaces[s].controls || [];
      for (var c = 0; c < controls.length; c += 1) {
        if (controls[c].focused) {
          return controls[c];
        }
      }
    }
    return null;
  }

  // Cursor identity, e.g. "T00 / LEVEL", derived from the focused control's
  // own label ("T00 Level"); falls back to the focus path's identity parts.
  function focusIdentity(model) {
    var control = focusedControl(model);
    if (control && control.label) {
      var label = String(control.label);
      var match = label.match(/^(T[0-9A-Fa-f]{2})\s+(.+)$/);
      if (match) {
        return match[1].toUpperCase() + " / " + match[2].toUpperCase();
      }
      return label.toUpperCase();
    }
    var id =
      model.focusPath && model.focusPath.controlId && model.focusPath.controlId.id;
    if (id && id.kind === "track") {
      return trackName(id.trackId) + " / " + String(id.parameter).toUpperCase();
    }
    return "";
  }

  // The focused mixer track id, or null — the meter rule and the Inspector
  // both key on it.
  function focusedTrackId(model) {
    var id =
      model.focusPath && model.focusPath.controlId && model.focusPath.controlId.id;
    if (id && (id.kind === "track" || id.kind === "send")) {
      return id.trackId;
    }
    var inspector = surfaceById(model, "mixerInspector");
    var summary = (inspector && inspector.summary) || null;
    return summary && summary.kind === "mixerInspector"
      ? summary.focusedTrack
      : null;
  }

  function statusToneClass(status) {
    if (!status) {
      return "muted";
    }
    if (status.kind === "ready") {
      return "positive";
    }
    return status.kind === "error" ? "warning" : "adjust";
  }

  // Condenses a valid action's label to the authored hint form:
  // "Move right" -> "right", "Open PATCH" -> "patch", "Adjust mode" -> "adjust".
  function hintLabel(action) {
    return String(action.label)
      .toLowerCase()
      .replace(/^(open|move)\s+/, "")
      .replace(/\s+mode$/, "");
  }

  // One projected action list as its hint run.
  //
  // The hints are separated by a text space, not by the flex gap alone. Every
  // container that holds a run is a flex container, so a whitespace-only text
  // node between two items generates no anonymous flex item and the painted
  // geometry is unchanged — but the run's `textContent` reads "2:patch 1:mixer"
  // rather than "2:patch1:mixer", which is what a screen reader announces, what
  // a selection copies, and what an observation can compare against the
  // projected list. Adjacent spans with no separating text made the run a
  // single unbroken word in every one of those readings.
  function hintRun(actions) {
    var spans = [];
    for (var i = 0; i < actions.length; i += 1) {
      var action = actions[i];
      if (!action.hint) {
        continue; // null hints never render (spike defect, kept fixed)
      }
      spans.push(
        '<span class="type-hint focus">' +
          escapeHtml(action.hint) +
          ":" +
          escapeHtml(hintLabel(action)) +
          "</span>"
      );
    }
    return spans.join(" ");
  }

  function actionsOfKind(model, kind) {
    var out = [];
    var actions = model.validActions || [];
    for (var i = 0; i < actions.length; i += 1) {
      if (actions[i].action && actions[i].action.kind === kind) {
        out.push(actions[i]);
      }
    }
    return out;
  }

  // ---- the shared control-state vocabulary --------------------------------

  // Derives the declared ComponentState treatment for one projected control
  // from the immutable document plus the projected interaction mode —
  // exactly the precedence the pre-cutover shipped row applied
  // (patch_strip_row::component_state): a failed edit outranks an in-flight
  // one, focus outranks read-only-ness. Matching is exhaustive over what the
  // document can carry; an unknown lifecycle kind or interaction mode is an
  // explicit visible `?state` marker, never a silent resting row.
  function controlState(control, mode) {
    if (control.error) {
      return { name: "error", raw: null };
    }
    var status = control.status || null;
    if (status) {
      var kind = String(status.kind);
      if (kind === "preparing" || kind === "activating") {
        return { name: "loading", raw: null };
      }
      if (kind !== "ready" && kind !== "failed") {
        return { name: "unknown", raw: kind };
      }
    }
    if (control.focused) {
      var modeName = String(mode);
      if (modeName === "adjust") {
        return { name: "adjusting", raw: null };
      }
      if (
        modeName === "navigate" ||
        modeName === "modal" ||
        modeName === "multiSelect"
      ) {
        return { name: "focused", raw: null };
      }
      return { name: "unknown", raw: modeName };
    }
    if (control.enabled && control.editable) {
      return { name: "resting", raw: null };
    }
    return { name: "disabled", raw: null };
  }

  // Renders one typed document value as finished text, mirroring the shipped
  // row's presentation (parameter_row::value_text): continuous values read to
  // three places, toggles read ON/OFF, identities and summaries read as
  // themselves, assets read their locator. An unknown value kind is an
  // explicit `?kind` marker, never an empty cell.
  function controlValueText(control) {
    var value = control && control.value;
    if (!value || typeof value !== "object") {
      return UNAVAILABLE_MARK;
    }
    if (value.kind === "scalar") {
      // A `stepped` control is a bounded integer — the MIDI channel and the
      // voice limit, the kind this mission's reducer newly produces. It rides
      // the same scalar envelope a continuous value does, so the row's own
      // declared kind is what tells them apart; without this a channel reads
      // "0.000" and a voice limit "64.000".
      return control.kind === "stepped"
        ? String(Math.round(Number(value.value)))
        : Number(value.value).toFixed(3);
    }
    if (value.kind === "parameter") {
      var parameter = value.value;
      if (parameter && typeof parameter === "object") {
        if (parameter.kind === "continuous") {
          return Number(parameter.value).toFixed(3);
        }
        if (parameter.kind === "stepped") {
          return String(parameter.value);
        }
        if (parameter.kind === "choice") {
          // The document's own authored name for the stored id, never a name
          // composed here. `parameter.value` is the choice *identity* —
          // `sf2.bank-0.program-40`, `braids.model.csaw` — and painting it is
          // the defect FR-014 closes; DESIGN.md calls this exact row "the
          // authored-name Preset row". The projection carries the descriptor's
          // label beside the id as `selectedLabel`, so the page reads a name it
          // was given (mission finding F-33). A choice with no projected label
          // falls back to the id rather than to a blank, because an unlabelled
          // option is a descriptor defect and hiding it would hide that.
          return String(
            control.selectedLabel === null || control.selectedLabel === undefined
              ? parameter.value
              : control.selectedLabel
          );
        }
        if (parameter.kind === "toggle") {
          return parameter.value ? "ON" : "OFF";
        }
        return "?" + String(parameter.kind);
      }
      return UNAVAILABLE_MARK;
    }
    if (value.kind === "asset") {
      return value.value && value.value.locator
        ? String(value.value.locator)
        : UNAVAILABLE_MARK;
    }
    if (value.kind === "identity" || value.kind === "summary") {
      return String(value.value);
    }
    return "?" + String(value.kind);
  }

  // The non-color signal a state carries beside its row (state.rs
  // NonColorSignal): Disabled says "Locked", Loading says the document's own
  // lifecycle word, Error says the typed failure text, an unknown state says
  // its explicit `?state` marker. Shape-signalled states return no text.
  function stateMarkHtml(control, state) {
    if (state.name === "disabled") {
      return '<span class="muted">Locked</span>';
    }
    if (state.name === "loading") {
      var word = control.status ? String(control.status.label) : "";
      return '<span class="adjust">' + escapeHtml(word) + "</span>";
    }
    if (state.name === "error") {
      var failure = control.error ? String(control.error.label) : "";
      return '<span class="warning">' + escapeHtml(failure) + "</span>";
    }
    if (state.name === "unknown") {
      return '<span class="warning">?' + escapeHtml(String(state.raw)) + "</span>";
    }
    return "";
  }

  // ---- band renderers (pure: document in, HTML string out) ---------------

  function contextLineHtml(model) {
    // The two declared top-level contexts; the active one carries the
    // authored "*" marker so activity is legible beyond color.
    var contexts = ["patch", "mixer"];
    var run = "";
    for (var i = 0; i < contexts.length; i += 1) {
      var name = contexts[i];
      var active = model.context === name;
      run +=
        '<span class="type-label context-entry' +
        (active ? " focus" : " muted") +
        '" data-context="' +
        name +
        '"' +
        (active ? ' data-active="true"' : "") +
        ">" +
        (active ? '<span class="patch">*</span> ' : "") +
        name.toUpperCase() +
        "</span>";
    }
    var status = model.status || null;
    var statusHtml = status
      ? '<span class="type-label ' +
        statusToneClass(status) +
        '" data-role="status">' +
        escapeHtml(status.label) +
        "</span>"
      : "";
    return (
      '<span class="type-heading">CREST SYNTH</span>' +
      '<span class="spring"></span>' +
      run +
      statusHtml
    );
  }

  // The identity band, one shared skeleton for both contexts: the context
  // display, a projected metadata run, and the focus annotation. Only the
  // metadata content differs, and it comes from the document's own surface
  // summaries, never from a page-side fork.
  function identityHeaderHtml(model, columns) {
    var metadata;
    if (model.context === "patch") {
      var main = surfaceById(model, "patchMain");
      var summary = (main && main.summary) || null;
      // The Patch's projected name and the engine's projected *label* — the
      // engine row's own value, which is the descriptor's authored label.
      // `summary.capabilityId` used to sit here; it is the capability's
      // identity, not its name, and an identity on screen is exactly the
      // defect FR-014 closes (see cross-WP finding F-28, which counts
      // `activeCapabilityId` a serialization key).
      var engine = controlById(main, "patch.engine");
      metadata =
        summary && summary.kind === "patch"
          ? escapeHtml(String(summary.patchName)) +
            HINT_SEPARATOR +
            escapeHtml(engine ? controlValueText(engine) : UNAVAILABLE_MARK)
          : UNAVAILABLE_MARK;
    } else {
      metadata = columns.length + " TRACKS";
    }
    return (
      '<span class="type-display">' +
      escapeHtml(String(model.context || "").toUpperCase()) +
      "</span>" +
      '<span class="type-label muted">/ ' +
      metadata +
      "</span>" +
      '<span class="spring"></span>' +
      '<span class="type-value focus" data-role="focus-annotation">' +
      escapeHtml(focusIdentity(model)) +
      "</span>"
    );
  }

  function columnHtml(column, model) {
    var focused = columnFocused(column);
    var correlated = focusedTrackId(model) === column.trackId;
    var focusedRow = columnFocusedRow(column);
    var mode = model.interactionMode;
    var state = faderState(column);
    var header =
      '<span class="structure track-header type-label ' +
      (focused ? "focus" : "secondary") +
      '" data-structure="TrackHeader">' +
      trackName(column.trackId) +
      "</span>";

    var fader;
    if (column.level) {
      var level = fraction(column.level).toFixed(6);
      fader =
        '<div class="structure level-fader" data-structure="LevelFader"' +
        ' data-control-row="level" data-row-state="' +
        mixerRowState(column.level, mode) +
        '" data-state="' +
        state +
        '" data-level="' +
        level +
        '">' +
        '<div class="fader-track"><div class="fader-fill"></div>' +
        '<div class="fader-cap"></div></div>' +
        '<div class="track-meter" data-meter-track="' +
        column.trackId +
        '" data-meter-state="stale" aria-label="' +
        trackName(column.trackId) +
        ' meter stale"><div class="track-meter-fill"></div></div></div>';
    } else {
      fader =
        '<div class="structure level-fader unavailable" data-structure="LevelFader"' +
        ' data-control-row="level" data-row-state="unavailable"' +
        ' data-state="unavailable"><span class="type-hint muted">' +
        UNAVAILABLE +
        '</span><div class="track-meter" data-meter-track="' +
        column.trackId +
        '" data-meter-state="stale" aria-label="' +
        trackName(column.trackId) +
        ' meter stale"><div class="track-meter-fill"></div></div></div>';
    }

    var readout = column.level
      ? '<span class="structure level-readout type-value ' +
        (column.level.focused ? "focus" : "patch") +
        '" data-structure="LevelReadout">' +
        midiHex(column.level) +
        "</span>"
      : '<span class="structure level-readout type-value muted unavailable"' +
        ' data-structure="LevelReadout">' +
        UNAVAILABLE +
        "</span>";

    var pan = column.pan
      ? '<span class="structure pan-readout type-hint" data-structure="PanReadout"' +
        ' data-control-row="pan" data-row-state="' +
        mixerRowState(column.pan, mode) +
        '">' +
        '<span class="muted">P</span> <span class="secondary">' +
        panCondition(column.pan) +
        "</span></span>"
      : '<span class="structure pan-readout type-hint muted unavailable"' +
        ' data-structure="PanReadout">P ' +
        UNAVAILABLE +
        "</span>";

    // Mute and solo together on one line, resting condition included, so an
    // engaged pair is never reduced to whichever was set last and a resting
    // line never reads as an unavailable one.
    var stateLine;
    if (column.mute || column.solo) {
      var mutePart = column.mute
        ? toggleOn(column.mute)
          ? '<span class="state-part warning" data-control-row="mute" data-row-state="' +
            mixerRowState(column.mute, mode) +
            '">M ON</span>'
          : '<span class="state-part muted" data-control-row="mute" data-row-state="' +
            mixerRowState(column.mute, mode) +
            '">M --</span>'
        : '<span class="muted">M ' + UNAVAILABLE + "</span>";
      var soloPart = column.solo
        ? toggleOn(column.solo)
          ? '<span class="state-part positive" data-control-row="solo" data-row-state="' +
            mixerRowState(column.solo, mode) +
            '">S ON</span>'
          : '<span class="state-part muted" data-control-row="solo" data-row-state="' +
            mixerRowState(column.solo, mode) +
            '">S --</span>'
        : '<span class="muted">S ' + UNAVAILABLE + "</span>";
      // The separator carries no surrounding spaces: the authored "M --" /
      // "S ON" marks plus a bare interpunct measure 72 px in the hint
      // style, which is what lets the line hold single-line inside the
      // authored 82 px desktop column (the spaced form measures 88 px and
      // cannot).
      stateLine =
        '<span class="structure state-line type-hint" data-structure="StateLine">' +
        mutePart +
        '<span class="muted">·</span>' +
        soloPart +
        "</span>";
    } else {
      stateLine =
        '<span class="structure state-line type-hint muted unavailable"' +
        ' data-structure="StateLine">M/S ' +
        UNAVAILABLE +
        "</span>";
    }

    return (
      '<div class="column' +
      (focused ? " focused" : correlated ? " correlated" : "") +
      '" data-track="' +
      column.trackId +
      '" data-focused-row="' +
      (focusedRow === null ? "" : focusedRow) +
      '">' +
      header +
      fader +
      readout +
      pan +
      stateLine +
      "</div>"
    );
  }

  // The meter element every workspace carries: render always writes the
  // zero/stale state (or nothing when no mixer track is focused) and only
  // the meter listener may overwrite it.
  function meterHtml(model) {
    return (
      '<span class="type-value secondary meter" id="meter-readout" data-meter-state="stale" aria-label="selected track meter stale">' +
      (focusedTrackId(model) === null ? "" : "METER 0.000") +
      "</span>"
    );
  }

  // The shared workspace scaffold both contexts ride: one caption row
  // (context-specific left run, the meter, a spring, a context-specific
  // right run, the navigate hints), the workspace body, and one mode hint
  // row. The rows are structural bands, not per-context forks.
  function workspaceScaffold(model, leftHtml, rightHtml, bodyHtml) {
    var navigate = actionsOfKind(model, "navigate");
    var mode = actionsOfKind(model, "setInteractionMode");
    return (
      '<div class="caption-row">' +
      leftHtml +
      meterHtml(model) +
      '<span class="spring"></span>' +
      rightHtml +
      hintRun(navigate) +
      "</div>" +
      bodyHtml +
      '<div class="hint-row">' +
      hintRun(mode) +
      '<span class="spring"></span>' +
      "</div>"
    );
  }

  function mixerWorkspaceHtml(model, columns) {
    var bank;
    if (columns.length > 0) {
      var cells = "";
      for (var i = 0; i < columns.length; i += 1) {
        cells += columnHtml(columns[i], model);
      }
      bank = '<div class="bank" id="bank">' + cells + "</div>";
    } else {
      // A bank with no track view data marks the bank unavailable rather
      // than painting a representative mixer (MixerStripBank rule).
      bank =
        '<div class="bank bank-unavailable" id="bank">' +
        '<span class="type-label muted">MIXER BANK ' +
        UNAVAILABLE +
        "</span></div>";
    }
    return workspaceScaffold(
      model,
      '<span class="type-label muted">LEVEL / PAN / MUTE / SOLO</span>',
      "",
      bank
    );
  }

  // ---- the PATCH strip ----------------------------------------------------

  // One projected range endpoint in the row's own presentation: a continuous
  // row reads its bounds to three places exactly as it reads its value, every
  // other kind reads its bounds as themselves. No bound is inferred and no
  // unit is formatted the document did not supply.
  function rangeEndpointText(control, value) {
    return control.kind === "continuous"
      ? Number(value).toFixed(3)
      : String(value);
  }

  // The projected bounds of a numeric row, or nothing at all when the
  // document carries none — an absent range is a projected fact, not a gap to
  // fill (FR-013).
  function rangeHtml(control) {
    var range = control && control.numericRange;
    if (
      !range ||
      typeof range.minimum !== "number" ||
      typeof range.maximum !== "number"
    ) {
      return "";
    }
    return (
      '<span class="prow-range type-hint muted" data-role="row-range">' +
      escapeHtml(rangeEndpointText(control, range.minimum)) +
      escapeHtml(RANGE_SEPARATOR) +
      escapeHtml(rangeEndpointText(control, range.maximum)) +
      "</span>"
    );
  }

  // Whether the capability declared this row read-only.
  //
  // This is `patchInteraction`, not `editable`. `editable` answers "would the
  // reducer accept an adjustment here, now" and is uniformly false on every
  // detail row in this phase, so it discriminates nothing about the
  // capability's own declaration; `patchInteraction` is that declaration,
  // projected from its single producer.
  function readOnly(control) {
    return control && control.patchInteraction === "readOnly";
  }

  // One projected control as a listed strip row: label left, the state's
  // non-color mark or the position indicator in the middle, then the rendered
  // value, its unit, its projected bounds, the capability's read-only
  // declaration, and the row's own valid actions — the shipped parameter-row
  // anatomy, widened by the facts FR-011/FR-013 project and nothing painted
  // it. The derived state rides the row as data plus a class so every
  // treatment resolves from the token vocabulary in page.css.
  function patchRowHtml(control, mode, role) {
    var state = controlState(control, mode);
    var id = controlIdOf(control);
    var focusPath = JSON.stringify(control.path || null);
    var semanticId =
      control && control.path && control.path.controlId
        ? control.path.controlId.id
        : null;
    var mixerAttributes = "";
    if (semanticId && typeof semanticId === "object") {
      mixerAttributes +=
        ' data-mixer-kind="' + escapeHtml(String(semanticId.kind || "")) + '"';
      if (semanticId.trackId !== undefined) {
        mixerAttributes += ' data-mixer-track="' + semanticId.trackId + '"';
      }
      if (semanticId.bus !== undefined) {
        mixerAttributes += ' data-mixer-bus="' + semanticId.bus + '"';
      }
      if (semanticId.parameter !== undefined) {
        mixerAttributes +=
          ' data-mixer-parameter="' +
          escapeHtml(String(semanticId.parameter)) +
          '"';
      }
    }
    var locked = readOnly(control);
    // A read-only row says READ-ONLY rather than the generic "Locked": the
    // declaration is the more specific fact and both are the same mark slot.
    var mark = locked ? "" : stateMarkHtml(control, state);
    var middle;
    if (mark) {
      middle =
        '<span class="prow-mark type-hint" data-role="state-mark">' +
        mark +
        "</span>" +
        '<span class="spring"></span>';
    } else {
      var position = positionFraction(control);
      middle =
        position === null
          ? '<span class="spring"></span>'
          : '<div class="prow-position"><div class="prow-position-fill"' +
            ' data-position="' +
            position.toFixed(6) +
            '"></div></div>';
    }
    var unit = control.unit
      ? '<span class="prow-unit type-hint muted">' +
        escapeHtml(String(control.unit)) +
        "</span>"
      : "";
    var lockedMark = locked
      ? '<span class="prow-readonly type-hint muted" data-role="read-only">' +
        READ_ONLY_MARK +
        "</span>"
      : "";
    // The row's own accepted actions, presented exactly as the footer
    // presents the model-level list — same function, same projected labels
    // and hints. At the focused row the two lists are the same value by
    // construction, so neither is special-cased against the other. The
    // Utility rows are excluded: the panel states its affordances once on its
    // own authored hint line, and a nine-hint run does not seat in a 320 px
    // side region.
    var hintRunHtml =
      role === "panel" ? "" : hintRun(control.validActions || []);
    var hints = hintRunHtml
      ? '<span class="prow-hints" data-role="row-hints">' +
        hintRunHtml +
        "</span>"
      : "";
    var row =
      '<div class="prow' +
      (role === "panel" ? " panel" : "") +
      '" data-control="' +
      escapeHtml(id) +
      '" data-focus-path="' +
      escapeHtml(focusPath) +
      '" data-state="' +
      state.name +
      '"' +
      mixerAttributes +
      (control.patchInteraction
        ? ' data-interaction="' + escapeHtml(String(control.patchInteraction)) + '"'
        : "") +
      ">" +
      '<span class="prow-label type-label">' +
      escapeHtml(String(control.label)) +
      "</span>" +
      middle +
      '<span class="prow-value type-value">' +
      escapeHtml(controlValueText(control)) +
      "</span>" +
      unit +
      rangeHtml(control) +
      lockedMark +
      hints +
      "</div>";
    return row + lifecycleHtml(control, id);
  }

  // The lifecycle band beneath a row reporting a structural edit (error,
  // in-flight status, or a requested value): the typed failure and/or
  // lifecycle word, the active and requested graph revisions, and the
  // requested value the edit is moving the row toward — the shipped row's
  // readout (patch_strip_row::render_lifecycle) with the one field it could
  // only mark unavailable now filled from the projection. It takes no focus
  // and appears only while the document reports the edit.
  //
  // This is the declared structural-edit vocabulary and stays the only one:
  // the active value never leaves the row above, so active, requested, and
  // status read together and the active graph stays explicit throughout
  // (FR-010).
  function lifecycleHtml(control, id) {
    var status = control.status || null;
    var inFlight =
      status && (status.kind === "preparing" || status.kind === "activating");
    var requested = control.requestedValue || null;
    if (!control.error && !inFlight && !requested) {
      return "";
    }
    var parts = [];
    if (control.error) {
      parts.push(String(control.error.label));
    }
    if (status) {
      parts.push(String(status.label));
      parts.push("ACTIVE GRAPH");
      parts.push(String(status.graphRevision));
      parts.push("REQUESTED GRAPH");
      parts.push(
        status.targetGraphRevision === null ||
          status.targetGraphRevision === undefined
          ? UNAVAILABLE_MARK
          : String(status.targetGraphRevision)
      );
    }
    // Rendered through the same value presentation the active value uses, so
    // the two read on the same terms. A row mid-edit that projects no
    // requested value is still marked rather than filled, so an absent one is
    // never mistaken for the active value under another name.
    parts.push("REQUESTED VALUE");
    parts.push(
      requested
        ? controlValueText({
            kind: control.kind,
            value: requested,
            // The requested value's own authored name, so the band never
            // paints a choice id beneath a row whose active value reads its
            // name (F-33, the requested half).
            selectedLabel: control.requestedLabel,
          })
        : UNAVAILABLE_MARK
    );
    var tone = control.error ? "warning" : "adjust";
    return (
      '<div class="lifecycle type-hint ' +
      tone +
      '" data-lifecycle-for="' +
      escapeHtml(id) +
      '">' +
      escapeHtml(parts.join(HINT_SEPARATOR)) +
      "</div>"
    );
  }

  // A designed structure the projection supplied no entry for: its static
  // name stays on screen and the authored row mark sits where its value
  // would be (section::mark_unavailable).
  function markUnavailableRowHtml(structure) {
    return (
      '<div class="prow unavailable" data-state="disabled">' +
      '<span class="prow-label type-label muted">' +
      escapeHtml(structure) +
      "</span>" +
      '<span class="spring"></span>' +
      '<span class="prow-value type-value muted">' +
      UNAVAILABLE_MARK +
      "</span>" +
      "</div>"
    );
  }

  // Which designed group one projected control belongs to, from its own
  // identity — the PATCH twin of the mixer bank keying its columns on
  // `id.trackId`. `openSlot` is the group the most recent occupancy row
  // opened: the projection emits a slot's occupant rows immediately after
  // that slot's occupancy row (PatchControlId::resolve), so an occupant joins
  // its own position without the page mapping slot instance ids to positions.
  // An identity no designed group claims returns null and is arranged in the
  // explicit unknown group rather than dropped.
  function stripGroupKey(id, openSlot) {
    if (id === "patch.engine") {
      return "instrument";
    }
    if (startsWith(id, "patch.envelope.")) {
      return "envelope";
    }
    if (startsWith(id, "patch.capability.")) {
      return "capability";
    }
    if (startsWith(id, "patch.effectSlot.")) {
      return "slot." + id.slice("patch.effectSlot.".length);
    }
    if (startsWith(id, "patch.effect.")) {
      return openSlot;
    }
    return null;
  }

  // The row that heads a group: the row whose projected *value* names what
  // the group holds. The instrument's parameter rows are headed by the engine
  // row; a slot's occupant rows by that slot's occupancy row. One mapping,
  // used by the strip (which nests the head row first) and by the detail
  // shell (whose title is the head row's value) — so neither branches on a
  // subject kind to learn what it is showing.
  function groupHeadControlId(key) {
    if (key === "instrument" || key === "capability") {
      return "patch.engine";
    }
    if (startsWith(key, "slot.")) {
      return "patch.effectSlot." + key.slice("slot.".length);
    }
    return null;
  }

  // Which strip group owns one control identity, answered by arranging the
  // main surface exactly as the strip arranges it. This is how the detail
  // shell learns what it is showing without reading `summary.subject`: a
  // detail row and its strip twin share an identity, and the strip already
  // knows which group that identity sits in.
  function ownerGroupKey(mainControls, id) {
    var groups = stripGroups(mainControls);
    for (var g = 0; g < groups.length; g += 1) {
      for (var r = 0; r < groups[g].rows.length; r += 1) {
        if (controlIdOf(groups[g].rows[r]) === id) {
          return groups[g].key;
        }
      }
    }
    return null;
  }

  function designedGroup(key) {
    for (var i = 0; i < DESIGNED_STRIP_GROUPS.length; i += 1) {
      if (DESIGNED_STRIP_GROUPS[i].key === key) {
        return DESIGNED_STRIP_GROUPS[i];
      }
    }
    return null;
  }

  // Arranges one surface's visible controls into ordered groups.
  //
  // Groups are created in first-appearance order and rows are appended in the
  // document's own order, so the painted order equals the projected order by
  // construction — grouping is a presentation change and the reducer keeps
  // the focus order. Every designed group the walk never opened is then
  // inserted at its declared position carrying no rows, so it can mark itself
  // unavailable inside its own group rather than vanishing.
  function stripGroups(controls) {
    var groups = [];
    var byKey = {};
    var openSlot = null;

    function group(key) {
      if (!Object.prototype.hasOwnProperty.call(byKey, key)) {
        var declared = designedGroup(key);
        byKey[key] = {
          key: key,
          legend: declared ? declared.legend : null,
          designed: declared ? declared.designed : false,
          unknown: false,
          rows: [],
        };
        groups.push(byKey[key]);
      }
      return byKey[key];
    }

    for (var i = 0; i < controls.length; i += 1) {
      var control = controls[i];
      if (!control.visible) {
        continue;
      }
      var id = controlIdOf(control);
      if (startsWith(id, "patch.effectSlot.")) {
        openSlot = stripGroupKey(id, null);
      }
      var key = stripGroupKey(id, openSlot);
      if (key === null) {
        // An identity no designed group claims: arranged under an explicit
        // marker, never silently dropped and never folded into a neighbour.
        var unknown = group("?group");
        unknown.unknown = true;
        unknown.rows.push(control);
        continue;
      }
      group(key).rows.push(control);
    }

    for (var d = DESIGNED_STRIP_GROUPS.length - 1; d >= 0; d -= 1) {
      var designed = DESIGNED_STRIP_GROUPS[d];
      if (!designed.designed || byKey[designed.key]) {
        continue;
      }
      // No rows, so where it lands cannot reorder any painted row; it lands
      // ahead of the first designed group that did appear after it, which is
      // its declared position.
      var at = groups.length;
      for (var g = 0; g < groups.length; g += 1) {
        var index = -1;
        for (var e = 0; e < DESIGNED_STRIP_GROUPS.length; e += 1) {
          if (DESIGNED_STRIP_GROUPS[e].key === groups[g].key) {
            index = e;
          }
        }
        if (index > d) {
          at = g;
          break;
        }
      }
      groups.splice(at, 0, {
        key: designed.key,
        legend: designed.legend,
        designed: true,
        unknown: false,
        rows: [],
      });
    }
    return groups;
  }

  // One group: its authored legend as a title band, then its rows. A designed
  // group the projection carried no row for marks that structure unavailable
  // *inside its own group*, so the strip never goes silent about a group it
  // could not draw (crest-spec PatchStrip, the two-level no-placeholder rule).
  function stripGroupHtml(group, mode, role, head) {
    // The authored legend where the design gives the structure one; otherwise
    // the projected *value* of the row that heads the group — the
    // instrument's own authored name over its parameter rows, which is the
    // same relationship a slot's occupancy row has to its occupant rows.
    var title = group.unknown
      ? "?" + group.key.slice(1)
      : group.legend || (head ? controlValueText(head) : null);
    var head = title
      ? '<div class="pgroup-title type-label muted" data-role="group-title">' +
        escapeHtml(String(title)) +
        "</div>"
      : "";
    var rows = "";
    for (var i = 0; i < group.rows.length; i += 1) {
      rows += patchRowHtml(group.rows[i], mode, role);
    }
    if (group.rows.length === 0) {
      rows = markUnavailableRowHtml(String(group.legend || group.key));
    }
    return (
      '<div class="pgroup' +
      (group.rows.length === 0 ? " unavailable" : "") +
      (group.unknown ? " unknown" : "") +
      '" data-group="' +
      escapeHtml(group.key) +
      '">' +
      head +
      '<div class="pgroup-rows">' +
      rows +
      "</div></div>"
    );
  }

  // The strip's own identity and routing header: which Patch is being edited
  // and where its sound goes. Informative, not focusable — it allocates no
  // interactive target and carries no control identity, so it enters no focus
  // order (the reducer's order is the projected control list, and this header
  // is not in it).
  //
  // Every value is projected: the Patch's name from the main surface's own
  // summary, the channel and the track from the two Utility rows that own
  // them, labels included. Nothing is re-derived here — re-deriving a label
  // is how a serialization key reached the screen in the first place.
  function stripHeaderHtml(model) {
    var main = surfaceById(model, "patchMain");
    var summary = (main && main.summary) || null;
    var side = persistentSideSurface(model);
    var name =
      summary && summary.kind === "patch"
        ? String(summary.patchName)
        : UNAVAILABLE_MARK;
    var parts =
      '<span class="strip-header-name type-heading" data-role="strip-patch-name">' +
      escapeHtml(name) +
      "</span>";
    var routing = ["patch.midiInput", "patch.output.outputTrack"];
    for (var i = 0; i < routing.length; i += 1) {
      var control = controlById(side, routing[i]);
      parts +=
        '<span class="strip-header-part type-hint muted" data-routing="' +
        escapeHtml(routing[i]) +
        '">' +
        (control
          ? escapeHtml(String(control.label)) +
            " " +
            escapeHtml(controlValueText(control))
          : UNAVAILABLE_MARK) +
        "</span>";
    }
    return (
      '<div class="strip-header" data-role="strip-header">' + parts + "</div>"
    );
  }

  // The PATCH main workspace as the declared `PatchStrip` composition: the
  // identity-and-routing header, then ordered groups each holding an optional
  // authored title and its rows — groups arranging groups, not one flat run
  // of every projected control. A workspace with no focused Patch at all
  // marks the strip unavailable; a group with no view data marks itself
  // (crest-spec PatchStrip).
  function patchStripHtml(model) {
    var main = surfaceById(model, "patchMain");
    var controls = (main && main.controls) || [];
    var mode = model.interactionMode;
    var groups = stripGroups(controls);
    var painted = 0;
    for (var g = 0; g < groups.length; g += 1) {
      painted += groups[g].rows.length;
    }
    if (painted === 0) {
      return (
        '<div class="strip strip-unavailable" id="strip">' +
        markUnavailableRowHtml("ENTRIES") +
        "</div>"
      );
    }
    var body = stripHeaderHtml(model);
    for (var i = 0; i < groups.length; i += 1) {
      body += stripGroupHtml(
        groups[i],
        mode,
        "listed",
        controlById(main, groupHeadControlId(groups[i].key))
      );
    }
    return '<div class="strip" id="strip">' + body + "</div>";
  }

  // The subordinate detail surface as the declared `CapabilityDetailShell`:
  // one composition arranging whichever capability the model names.
  //
  // It branches on no subject kind. The title is the projected value of the
  // strip row that owns these rows — the engine row for an instrument
  // subject, the slot's occupancy row for an effect subject — resolved
  // through `groupHeadControlId` from the *control identities the detail
  // surface carries*, never from `summary.subject`. That value is the
  // capability's authored label; `summary.subject.capabilityId` is its
  // identity, and an identity on screen is the defect FR-014 closes.
  //
  // The surface carries one group's worth of rows: the semantic model
  // projects the capability's parameters as a flat ordered list and carries
  // no section identity, so the shell arranges the groups the projection
  // distinguishes, which is one. Section order therefore is the projected
  // order.
  function detailShellHtml(model) {
    var detail = surfaceByRole(model, "detail");
    var controls = (detail && detail.controls) || [];
    var mode = model.interactionMode;
    var main = surfaceById(model, "patchMain");
    var head = controls.length
      ? controlById(
          main,
          groupHeadControlId(
            ownerGroupKey((main && main.controls) || [], controlIdOf(controls[0]))
          )
        )
      : null;
    var subject = head ? controlValueText(head) : UNAVAILABLE_MARK;
    // A capability mid-preparation reports its projected lifecycle word here
    // as well as on its rows, so the shell is never read as a settled one.
    var status = null;
    for (var s = 0; s < controls.length; s += 1) {
      if (controls[s].status) {
        status = controls[s].status;
        break;
      }
    }
    var sections = "";
    var declaredSections = (detail && detail.sections) || [];
    for (var sectionIndex = 0; sectionIndex < declaredSections.length; sectionIndex += 1) {
      var declared = declaredSections[sectionIndex];
      var sectionRows = "";
      var paths = declared.controlPaths || [];
      for (var pathIndex = 0; pathIndex < paths.length; pathIndex += 1) {
        var wanted = JSON.stringify(paths[pathIndex]);
        for (var rowIndex = 0; rowIndex < controls.length; rowIndex += 1) {
          if (controls[rowIndex].visible && JSON.stringify(controls[rowIndex].path) === wanted) {
            sectionRows += patchRowHtml(controls[rowIndex], mode, "detail");
            break;
          }
        }
      }
      sections +=
        '<section class="detail-section" data-detail-section="' +
        escapeHtml(String(declared.id)) +
        '"><h3 class="type-label muted" data-role="detail-section-label">' +
        escapeHtml(String(declared.label)) +
        "</h3>" +
        (sectionRows || markUnavailableRowHtml(String(declared.label))) +
        "</section>";
    }
    if (!sections) {
      var rows = "";
      for (var i = 0; i < controls.length; i += 1) {
        if (controls[i].visible) {
          rows += patchRowHtml(controls[i], mode, "detail");
        }
      }
      sections =
        '<section class="detail-section" data-detail-section="unsectioned">' +
        (rows || markUnavailableRowHtml(String((detail && detail.label) || "DETAIL"))) +
        "</section>";
    }
    var visualizationMarkup = "";
    var visualizations = (detail && detail.visualizations) || [];
    for (var visualizationIndex = 0; visualizationIndex < visualizations.length; visualizationIndex += 1) {
      visualizationMarkup += detailVisualizationHtml(visualizations[visualizationIndex]);
    }
    return (
      '<div class="detail" id="detail">' +
      '<div class="detail-title" data-role="detail-title">' +
      '<span class="type-label muted">' +
      escapeHtml(String((detail && detail.label) || "DETAIL")) +
      "</span>" +
      '<span class="type-heading focus" data-role="detail-subject">' +
      escapeHtml(subject) +
      "</span>" +
      (status
        ? '<span class="type-hint adjust" data-role="detail-status">' +
          escapeHtml(String(status.label)) +
          "</span>"
        : "") +
      "</div>" +
      '<div class="detail-sections" data-role="detail-sections">' +
      sections +
      "</div>" +
      '<div class="detail-visualizations" data-role="detail-visualizations">' +
      visualizationMarkup +
      "</div></div>"
    );
  }

  function detailVisualizationHtml(visualization) {
    var data = visualization.data || { kind: "status", text: UNAVAILABLE };
    var body = "";
    if (data.kind === "envelope") {
      body =
        '<div class="envelope-shape" aria-hidden="true"><span></span><span></span><span></span><span></span></div>' +
        '<span class="type-hint">A ' +
        escapeHtml(Number(data.attackMilliseconds || 0).toFixed(1)) +
        " / D " +
        escapeHtml(Number(data.decayMilliseconds || 0).toFixed(1)) +
        " / S " +
        escapeHtml(Number(data.sustain || 0).toFixed(3)) +
        " / R " +
        escapeHtml(Number(data.releaseMilliseconds || 0).toFixed(1)) +
        "</span>";
    } else if (data.kind === "waveform") {
      var pairs = data.pairs || [];
      var bars = "";
      var stride = Math.max(1, Math.ceil(pairs.length / 192));
      for (var pairIndex = 0; pairIndex < pairs.length; pairIndex += stride) {
        var pair = pairs[pairIndex];
        var height = Math.max(
          0.02,
          Math.min(1, Number(pair.leftMax || 0) - Number(pair.leftMin || 0))
        );
        bars += '<i style="--wave-height:' + height.toFixed(6) + '"></i>';
      }
      var landmarks = "";
      var landmarkValues = data.landmarks || [];
      for (var landmarkIndex = 0; landmarkIndex < landmarkValues.length; landmarkIndex += 1) {
        var landmark = landmarkValues[landmarkIndex];
        landmarks +=
          '<b data-landmark="' +
          escapeHtml(String(landmark.role)) +
          '" style="--landmark-position:' +
          Math.max(0, Math.min(1, Number(landmark.normalizedPosition || 0))).toFixed(6) +
          '"><span class="type-hint">' +
          escapeHtml(String(landmark.role).toUpperCase()) +
          "</span></b>";
      }
      body =
        '<div class="waveform-shape" data-role="waveform-shape">' +
        (bars || '<span class="type-hint muted">NO PREPARED SUMMARY</span>') +
        landmarks +
        "</div>" +
        '<span class="type-hint" data-role="waveform-status">' +
        escapeHtml(String(data.status || UNAVAILABLE)) +
        "</span>";
    } else {
      body = '<span class="type-hint">' + escapeHtml(String(data.text || UNAVAILABLE)) + "</span>";
    }
    return (
      '<figure class="detail-visualization" data-visualization="' +
      escapeHtml(String(visualization.id)) +
      '" data-visualization-kind="' +
      escapeHtml(String(data.kind)) +
      '"><figcaption class="type-label muted">' +
      escapeHtml(String(visualization.label)) +
      "</figcaption>" +
      body +
      "</figure>"
    );
  }

  // One shared subordinate modal composition for descriptor choices and the
  // Sample Browser. Row identity, current/focus markers, lifecycle, and hold
  // state are all projected; the DOM owns no option or file index.
  function modalShellHtml(model) {
    var modal = surfaceByRole(model, "modal");
    if (!modal) {
      return "";
    }
    var summary = modal.summary || {};
    var browser = summary.kind === "sampleBrowser";
    var controls = modal.controls || [];
    var rows = "";
    for (var i = 0; i < controls.length; i += 1) {
      var control = controls[i];
      if (control.visible !== true) {
        continue;
      }
      var marker = [];
      if (control.selectedLabel) {
        marker.push(String(control.selectedLabel));
      }
      if (control.focused) {
        marker.push("FOCUS");
      }
      if (!control.enabled) {
        marker.push(UNAVAILABLE);
      }
      var browserMetadata = control.browserMetadata || null;
      if (browserMetadata && browserMetadata.status === "pending") {
        marker.push("LOADING");
      } else if (browserMetadata && browserMetadata.status === "failed") {
        marker.push("INVALID");
      }
      rows +=
        '<div class="modal-option' +
        (control.focused ? " is-focused" : "") +
        (!control.enabled ? " is-disabled" : "") +
        (browserMetadata ? " has-browser-metadata" : "") +
        '" data-focus-path="' +
        escapeHtml(JSON.stringify(control.path || null)) +
        '" data-metadata-state="' +
        escapeHtml(String((browserMetadata && browserMetadata.status) || "none")) +
        '">' +
        '<span class="modal-option-shape" aria-hidden="true">' +
        (control.focused ? "▶" : "◇") +
        "</span>" +
        '<span class="type-value modal-option-label">' +
        escapeHtml(String(control.label || UNAVAILABLE_MARK)) +
        "</span>" +
        (browserMetadata || control.unit
          ? '<span class="type-hint muted modal-option-meta">' +
            escapeHtml(String(browserMetadata ? browserMetadata.text : control.unit)) +
            "</span>"
          : "") +
        '<span class="type-hint modal-option-state">' +
        escapeHtml(marker.join(HINT_SEPARATOR) || "AVAILABLE") +
        "</span></div>";
    }
    if (!rows) {
      rows = markUnavailableRowHtml(browser ? "FILES" : "OPTIONS");
    }
    var status = "";
    if (browser) {
      var browserVisualizations = "";
      var modalVisualizations = modal.visualizations || [];
      for (var visualizationIndex = 0; visualizationIndex < modalVisualizations.length; visualizationIndex += 1) {
        browserVisualizations += detailVisualizationHtml(modalVisualizations[visualizationIndex]);
      }
      var preview = summary.preview || { kind: "idle" };
      var previewText = String(preview.kind || "idle").toUpperCase();
      if (preview.assetId) {
        previewText += HINT_SEPARATOR + String(preview.assetId);
      }
      if (preview.kind === "preparing") {
        previewText += preview.held ? HINT_SEPARATOR + "HELD" : HINT_SEPARATOR + "RELEASED";
      }
      status =
        '<div class="browser-status" data-role="browser-status">' +
        '<span class="type-label muted">' +
        escapeHtml(String(summary.folder || "/")) +
        "</span>" +
        '<span class="type-hint">' +
        escapeHtml(String(summary.lifecycle || UNAVAILABLE).toUpperCase()) +
        "</span>" +
        '<span class="type-hint adjust">PREVIEW ' +
        escapeHtml(previewText) +
        "</span>" +
        '<span class="type-hint muted">ORIGIN PATCH ROUTING / MUTE / SOLO / SENDS APPLY</span>' +
        "</div>" +
        '<div class="browser-visualizations" data-role="browser-visualizations">' +
        (browserVisualizations || markUnavailableRowHtml("WAVEFORM")) +
        "</div>" +
        '<div class="preview-region" data-role="preview-region" data-preview-state="' +
        escapeHtml(String(preview.kind || "idle")) +
        '" aria-label="preview ' +
        escapeHtml(previewText.toLowerCase()) +
        '">' +
        '<span class="type-label muted">PREVIEW</span>' +
        '<span class="preview-playhead-shape" aria-hidden="true"><span class="preview-playhead-marker"></span></span>' +
        '<span class="type-hint" data-role="preview-playhead">PLAYHEAD — / ' +
        escapeHtml(String(preview.kind || "idle").toUpperCase()) +
        "</span></div>";
    }
    return (
      '<div class="modal-shell' + (browser ? " sample-browser" : " option-modal") + '" id="modal-shell">' +
      '<div class="modal-title">' +
      '<span class="type-label muted">' +
      escapeHtml(browser ? "LIBRARY" : "SELECT OPTION") +
      "</span>" +
      '<span class="type-heading focus">' +
      escapeHtml(String(modal.label || (browser ? "SAMPLE BROWSER" : "OPTIONS"))) +
      "</span></div>" +
      status +
      '<div class="modal-options" data-role="modal-options">' +
      rows +
      "</div></div>"
    );
  }

  // The PATCH main workspace: the section header content (the surface's own
  // label left, the focused entry's annotation right) on the shared caption
  // row, then the composition the document's own surface set selects — the
  // detail shell while a detail entry is open, the grouped strip otherwise.
  // The shell bands and the persistent side region are untouched either way.
  function patchWorkspaceHtml(model) {
    var main = surfaceById(model, "patchMain");
    var detail = surfaceByRole(model, "detail");
    var modal = surfaceByRole(model, "modal");
    var body = modal
      ? modalShellHtml(model)
      : detail
        ? detailShellHtml(model)
        : patchStripHtml(model);
    var focused = focusedControl(model);
    var annotation = focused
      ? '<span class="type-hint focus" data-role="section-annotation">FOCUS' +
        HINT_SEPARATOR +
        escapeHtml(String(focused.label)) +
        "</span>"
      : "";
    var surface = modal || detail || main;
    var title = escapeHtml(String((surface && surface.label) || "PATCH"));
    return workspaceScaffold(
      model,
      '<span class="type-label muted">' + title + "</span>",
      annotation,
      body
    );
  }

  // ---- the persistent side region -----------------------------------------

  // Dispatches on the document's own side-surface summary — the Inspector
  // reading for a mixerInspector summary, the Utility reading for a
  // patchUtility one. A missing side surface says so; an incoherent summary
  // is marked, never guessed at.
  function sideRegionHtml(model) {
    var side = persistentSideSurface(model);
    if (!side) {
      // The persistent side region with no surface behind it says so; it is
      // never painted with representative content.
      return (
        '<span class="type-label muted">CURSOR</span>' +
        '<span class="type-value muted" data-role="cursor">' +
        UNAVAILABLE +
        "</span>"
      );
    }
    var summary = side.summary || {};
    if (summary.kind === "mixerInspector") {
      return mixerInspectorHtml(model, side);
    }
    if (summary.kind === "patchUtility") {
      return patchUtilityHtml(model, side, summary);
    }
    // A main-surface summary in the side region is incoherent: the panel has
    // no reading for it and will not invent one.
    return (
      '<span class="type-label muted">PANEL SUMMARY</span>' +
      '<span class="type-value muted">' +
      UNAVAILABLE_MARK +
      "</span>"
    );
  }

  // The panel's own authored hint line (design file `Utility Panel` 36:52,
  // "D-pad Right:enter   D-pad Left:return"): how an operator enters the panel
  // and how they leave it.
  //
  // Both facts are projected, one on each side of the boundary — the action
  // that enters this surface rides the rows an operator enters it *from*, and
  // the action that leaves it rides the panel's own rows. So the line is
  // gathered by walking the document in its declared order and keeping the
  // `enterSurface`-into-this-surface and `return` actions, deduplicated. The
  // page composes no action vocabulary of its own: the same `hintRun` renders
  // the same projected hints and labels the footer renders.
  function sideRegionHintLine(model, surface) {
    var actions = [];
    var seen = {};
    var surfaces = model.surfaces || [];
    for (var s = 0; s < surfaces.length; s += 1) {
      var controls = surfaces[s].controls || [];
      for (var c = 0; c < controls.length; c += 1) {
        var valid = controls[c].validActions || [];
        for (var a = 0; a < valid.length; a += 1) {
          var action = valid[a];
          var kind = action.action && action.action.kind;
          var entersThis =
            kind === "enterSurface" &&
            action.action.payload === (surface && surface.id);
          var leavesThis = kind === "return" && surfaces[s].id === surface.id;
          if (!entersThis && !leavesThis) {
            continue;
          }
          var key = String(action.hint) + "\u0000" + String(action.label);
          if (seen[key]) {
            continue;
          }
          seen[key] = true;
          actions.push(action);
        }
      }
    }
    if (actions.length === 0) {
      return "";
    }
    return (
      '<span class="panel-hint" data-role="utility-hint">' +
      hintRun(actions) +
      "</span>"
    );
  }

  // The PATCH Utility panel: the surface title, the projected patch identity
  // caption, the authored hint line, then the five designed entries in
  // authored order — each backed by its projected driver row or marked
  // explicitly unavailable — then any remaining projected entries the
  // designed set does not name
  // (utility_inspector_panel::render_patch_utility).
  //
  // The row set is bounded by declaration at five, so the panel seats its
  // entries within the side region with no scroll affordance; `#inspector`
  // keeps `overflow: hidden` and the rows are sized to fit rather than the
  // region relaxed to admit them (crest-spec UtilityInspectorPanel).
  function patchUtilityHtml(model, surface, summary) {
    var mode = model.interactionMode;
    var controls = surface.controls || [];
    var byId = {};
    for (var i = 0; i < controls.length; i += 1) {
      var control = controls[i];
      var id = controlIdOf(control);
      byId[id] = control;
    }
    var rows = "";
    var claimed = {};
    for (var d = 0; d < DESIGNED_UTILITY_ENTRIES.length; d += 1) {
      var entry = DESIGNED_UTILITY_ENTRIES[d];
      var driven =
        entry.driver !== null &&
        Object.prototype.hasOwnProperty.call(byId, entry.driver) &&
        byId[entry.driver].visible
          ? byId[entry.driver]
          : null;
      if (driven) {
        claimed[entry.driver] = true;
        rows += patchRowHtml(driven, mode, "panel");
      } else {
        rows += markUnavailableRowHtml(entry.label);
      }
    }
    for (var r = 0; r < controls.length; r += 1) {
      var remaining = controls[r];
      var remainingId = controlIdOf(remaining);
      if (claimed[remainingId] || !remaining.visible) {
        continue;
      }
      rows += patchRowHtml(remaining, mode, "panel");
    }
    // The authored identity line (design file 36:51). The Patch number is
    // projected by this surface's own summary; the name beside it is the main
    // surface's projected Patch name. `summary.capabilityId` used to stand
    // there — the capability's identity, not its name, which is the class of
    // value FR-014 keeps off the screen.
    var main = surfaceById(model, "patchMain");
    var mainSummary = (main && main.summary) || null;
    var identity =
      '<span class="type-hint focus" data-role="patch-identity">' +
      escapeHtml(String(summary.patchId)) +
      HINT_SEPARATOR +
      escapeHtml(
        mainSummary && mainSummary.kind === "patch"
          ? String(mainSummary.patchName)
          : UNAVAILABLE_MARK
      ) +
      "</span>";
    return (
      '<span class="type-label muted">' +
      escapeHtml(String(surface.label || "UTILITY")) +
      "</span>" +
      identity +
      sideRegionHintLine(model, surface) +
      rows
    );
  }

  function mixerIdentity(control, id) {
    if (control && control.label) {
      var label = String(control.label);
      var match = label.match(/^(T[0-9A-Fa-f]{2})\s+(.+)$/);
      return match
        ? match[1].toUpperCase() + " / " + match[2].toUpperCase()
        : label.toUpperCase();
    }
    return id && id.kind === "track"
      ? trackName(id.trackId) + " / " + String(id.parameter).toUpperCase()
      : UNAVAILABLE;
  }

  // The persistent MIXER Inspector: a correlation header bound to the
  // selected main-track control and a separately scrollable body composed
  // from the Inspector surface's canonical FocusPath-ordered controls.
  function mixerInspectorHtml(model, inspector) {
    var main = surfaceById(model, "mixerMain");
    var summary = inspector.summary || {};
    var trackId = focusedTrackId(model);
    var correlatedId = summary.focusedControl || null;
    var focused = controlById(main, JSON.stringify(correlatedId || null));
    var columns = trackColumns(main);
    var column = null;
    for (var i = 0; i < columns.length; i += 1) {
      if (columns[i].trackId === trackId) {
        column = columns[i];
      }
    }

    // Big readout: the focused control's value in its bound presentation
    // form — the focused level reads in the LevelReadout's MidiHexadecimal
    // binding; a toggle reads ON/OFF; any other continuous reads 0–127.
    var big = UNAVAILABLE;
    var bigTone = "muted";
    var rangeHint = "";
    if (focused) {
      var parameter =
        focused.path.controlId.id && focused.path.controlId.id.parameter;
      if (focused.kind === "toggle") {
        big = toggleOn(focused) ? "ON" : "OFF";
        bigTone = toggleOn(focused) ? "positive" : "secondary";
      } else if (parameter === "level") {
        big = midiHex(focused);
        bigTone = "positive";
        rangeHint = "LEVEL 00 — 7F";
      } else if (parameter === "pan") {
        big = panCondition(focused);
        bigTone = "positive";
        rangeHint = "PAN L — C — R";
      } else if (focused.numericRange) {
        big = midiDecimal(focused);
        bigTone = "positive";
        rangeHint = "0 — 127";
      }
    }

    var muteSolo = "";
    if (column && (column.mute || column.solo)) {
      var muteValue = column.mute
        ? toggleOn(column.mute)
          ? '<span class="warning">ON</span>'
          : "OFF"
        : UNAVAILABLE;
      var soloValue = column.solo
        ? toggleOn(column.solo)
          ? '<span class="positive">ON</span>'
          : "OFF"
        : UNAVAILABLE;
      muteSolo =
        '<div class="rule"></div>' +
        '<table class="type-hint secondary" data-role="mute-solo">' +
        "<tbody>" +
        '<tr><td>MUTE</td><td data-role="mute">' +
        muteValue +
        "</td></tr>" +
        '<tr><td>SOLO</td><td data-role="solo">' +
        soloValue +
        "</td></tr>" +
        "</tbody></table>";
    }

    var routes = summary.routedPatches || [];
    var routeRows = "";
    for (var p = 0; p < routes.length; p += 1) {
      routeRows +=
        '<li data-route="' +
        p +
        '" data-patch-id="' +
        escapeHtml(String(routes[p].patchId)) +
        '" data-patch-name="' +
        escapeHtml(String(routes[p].patchName)) +
        '"><span class="route-id">P' +
        String(routes[p].patchId).padStart(2, "0") +
        '</span><span class="route-name">' +
        escapeHtml(String(routes[p].patchName)) +
        "</span></li>";
    }
    var routing = routeRows
      ? '<ul class="route-list type-hint secondary" data-role="routes">' +
        routeRows +
        "</ul>"
      : '<span class="type-hint muted empty-route" data-role="routes">EMPTY</span>';

    var controlRows = "";
    var controls = inspector.controls || [];
    for (var c = 0; c < controls.length; c += 1) {
      if (controls[c].visible) {
        controlRows += patchRowHtml(controls[c], model.interactionMode, "panel");
      }
    }

    return (
      '<div class="inspector-pinned" data-role="inspector-correlation">' +
      '<span class="type-label muted">CURSOR</span>' +
      '<span class="type-value focus" data-role="cursor">' +
      escapeHtml(mixerIdentity(focused, correlatedId)) +
      "</span>" +
      '<div class="inspector-reading"><span class="type-display big-readout ' +
      bigTone +
      '" data-role="big-readout">' +
      escapeHtml(big) +
      "</span>" +
      '<span class="type-value secondary inspector-meter" id="inspector-meter-readout" data-meter-state="stale">METER 0.000 / STALE</span></div>' +
      (rangeHint
        ? '<span class="type-hint muted" data-role="readout-range">' +
          escapeHtml(rangeHint) +
          "</span>"
        : "") +
      muteSolo +
      '<div class="rule"></div><span class="type-label muted">ROUTING</span>' +
      routing +
      "</div>" +
      '<div class="inspector-controls" data-role="inspector-controls">' +
      sideRegionHintLine(model, inspector) +
      controlRows +
      "</div>"
    );
  }

  function footerHtml(model) {
    var breadcrumb = String(model.context || "").toUpperCase();
    var identity = focusIdentity(model);
    if (identity) {
      breadcrumb += " / " + identity;
    }
    return (
      '<span class="type-hint secondary" data-role="breadcrumb">' +
      escapeHtml(breadcrumb) +
      "</span>" +
      '<span class="spring"></span>' +
      hintRun(model.validActions || [])
    );
  }

  // ---- the pure render ---------------------------------------------------

  // CSP: the production policy (style-src 'self', no unsafe-inline) blocks
  // every parsed inline style attribute, but CSSOM property assignment is
  // exempt by spec — dynamic geometry must go through here. The rendered
  // HTML carries the value as a data attribute (data-level / data-position);
  // this one pass reads those attributes back and applies the custom
  // properties page.css consumes. A pure function of the DOM — no state, no
  // memory of prior renders — so the same document still paints the same
  // geometry (determinism proof target). Absence of the attribute means "no
  // data", never "zero": unavailable branches emit no attribute and get no
  // property.
  function applyDynamicGeometry(doc) {
    var levels = doc.querySelectorAll("[data-level]");
    for (var i = 0; i < levels.length; i += 1) {
      levels[i].style.setProperty(
        "--level",
        levels[i].getAttribute("data-level")
      );
    }
    var positions = doc.querySelectorAll("[data-position]");
    for (var j = 0; j < positions.length; j += 1) {
      positions[j].style.setProperty(
        "--position",
        positions[j].getAttribute("data-position")
      );
    }
  }

  // Focus visibility follows the serialized semantic path. The DOM is
  // searched by identity after every paint/reflow; no row number becomes
  // application state and scrolling dispatches no semantic event.
  function revealSemanticFocus(doc, model) {
    if (!model) {
      return;
    }
    var wanted = JSON.stringify(model.focusPath || null);
    var rows = doc.querySelectorAll('[data-focus-path]');
    for (var i = 0; i < rows.length; i += 1) {
      if (rows[i].getAttribute("data-focus-path") === wanted) {
        rows[i].scrollIntoView({ block: "nearest", inline: "nearest" });
        return;
      }
    }
  }

  // Rebuilds the five shell bands from one deserialized
  // SemanticGraphicalViewModel document. Same document, identical DOM. The
  // context line, identity header, workspace scaffold, side region, and
  // footer are the same shared bands for both contexts; only the workspace
  // body and the side reading follow the document's own context.
  function render(model) {
    var doc = window.document;
    latestModel = model;
    var main = surfaceById(model, "mixerMain");
    var columns = trackColumns(main);
    doc.getElementById("context-line").innerHTML = contextLineHtml(model);
    doc.getElementById("identity-header").innerHTML = identityHeaderHtml(
      model,
      columns
    );
    doc.getElementById("workspace").innerHTML =
      model.context === "patch"
        ? patchWorkspaceHtml(model)
        : mixerWorkspaceHtml(model, columns);
    doc.getElementById("inspector").innerHTML = sideRegionHtml(model);
    doc.getElementById("footer").innerHTML = footerHtml(model);
    // Final step, after ALL five region insertions: apply the dynamic
    // geometry in the same paint, on the initial render and every re-render
    // alike. render() is the single place projection content enters the DOM.
    applyDynamicGeometry(doc);
    revealSemanticFocus(doc, model);
  }

  // ---- the structural observation (acceptance harness contract) -----------

  // Renders the document, then reads the painted DOM back into post-paint
  // structural evidence. Everything in the returned object is copied from
  // what was actually painted — not from the input document — so a plan that
  // never reached the screen cannot satisfy it. The MIXER fields (columns,
  // inspector, meter) and the PATCH fields (rows, utility, annotations) are
  // both always present; whichever context did not paint reports its empty
  // shape, deterministically.
  //
  // Two calls with one document at one window size return deep-equal
  // objects; inspector.widthPx is asserted >= 320 at the compact viewport.
  function renderObservation(model) {
    render(model);
    var doc = window.document;

    function painted(id) {
      var el = doc.getElementById(id);
      if (!el) {
        return false;
      }
      var rect = el.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    }

    function textOf(root, selector) {
      var el = root.querySelector(selector);
      return el ? el.textContent.replace(/\s+/g, " ").trim() : null;
    }

    var bands = {
      contextLine: painted("context-line"),
      identityHeader: painted("identity-header"),
      workspace: painted("workspace"),
      inspector: painted("inspector"),
      footer: painted("footer"),
    };

    var columns = [];
    var columnNodes = doc.querySelectorAll("#bank .column");
    for (var i = 0; i < columnNodes.length; i += 1) {
      var node = columnNodes[i];
      var structures = [];
      var structureNodes = node.querySelectorAll("[data-structure]");
      for (var s = 0; s < structureNodes.length; s += 1) {
        structures.push(structureNodes[s].getAttribute("data-structure"));
      }
      var columnMeter = node.querySelector("[data-meter-track]");
      columns.push({
        trackId: Number(node.getAttribute("data-track")),
        header: textOf(node, '[data-structure="TrackHeader"]'),
        structures: structures,
        focused: node.classList.contains("focused"),
        correlated: node.classList.contains("correlated"),
        focusedRow: node.getAttribute("data-focused-row") || null,
        levelHex: textOf(node, '[data-structure="LevelReadout"]'),
        pan: textOf(node, '[data-structure="PanReadout"]'),
        stateLine: textOf(node, '[data-structure="StateLine"]'),
        rowStates: Array.prototype.map.call(
          node.querySelectorAll("[data-control-row]"),
          function (row) {
            return {
              row: row.getAttribute("data-control-row"),
              state: row.getAttribute("data-row-state"),
            };
          }
        ),
        meterState: columnMeter
          ? columnMeter.getAttribute("data-meter-state")
          : null,
        meterLabel: columnMeter
          ? columnMeter.getAttribute("aria-label")
          : null,
      });
    }

    // The painted width of one element inside a row, or null when the row does
    // not carry it. Rounded, so two renders of one document at one window size
    // report the identical integer.
    function widthOf(root, selector) {
      var el = root.querySelector(selector);
      return el ? Math.round(el.getBoundingClientRect().width) : null;
    }

    // The painted top and bottom edges of one element, in the row's own
    // coordinates — enough to see which of a wrapping row's lines it landed
    // on, which is the fact a width alone cannot report.
    function edgesIn(row, selector) {
      var el = row.querySelector(selector);
      if (!el) {
        return null;
      }
      var box = el.getBoundingClientRect();
      var origin = row.getBoundingClientRect().top;
      return {
        topPx: Math.round(box.top - origin),
        bottomPx: Math.round(box.bottom - origin),
      };
    }

    // The painted PATCH strip rows, in painted order, with the state each
    // row was painted in — the PATCH twin of the mixer column report.
    //
    // The painted box widths ride along because the row's parts compete for
    // one line: a hint run that takes the rail's width leaves the rail a few
    // pixels, which is a layout fact no text-only observation can see.
    function rowReport(nodes) {
      var out = [];
      for (var r = 0; r < nodes.length; r += 1) {
        var rowNode = nodes[r];
        out.push({
          control: rowNode.getAttribute("data-control"),
          state: rowNode.getAttribute("data-state"),
          label: textOf(rowNode, ".prow-label"),
          value: textOf(rowNode, ".prow-value"),
          mark: textOf(rowNode, '[data-role="state-mark"]'),
          unit: textOf(rowNode, ".prow-unit"),
          range: textOf(rowNode, '[data-role="row-range"]'),
          hints: textOf(rowNode, '[data-role="row-hints"]'),
          interaction: rowNode.getAttribute("data-interaction"),
          readOnly: textOf(rowNode, '[data-role="read-only"]'),
          heightPx: Math.round(rowNode.getBoundingClientRect().height),
          railPx: widthOf(rowNode, ".prow-position"),
          hintsPx: widthOf(rowNode, '[data-role="row-hints"]'),
          labelEdges: edgesIn(rowNode, ".prow-label"),
          hintEdges: edgesIn(rowNode, '[data-role="row-hints"]'),
        });
      }
      return out;
    }
    var rows = rowReport(doc.querySelectorAll("#strip .prow"));

    // The painted group anatomy of the PATCH strip, in painted order: the
    // structural evidence that the workspace arranges groups rather than one
    // flat row run, and that a group with no view data marked itself inside
    // its own group instead of vanishing.
    var groups = [];
    var groupNodes = doc.querySelectorAll("#strip .pgroup");
    for (var gi = 0; gi < groupNodes.length; gi += 1) {
      var groupNode = groupNodes[gi];
      var groupRowNodes = groupNode.querySelectorAll(".prow");
      var groupRows = [];
      for (var gr = 0; gr < groupRowNodes.length; gr += 1) {
        groupRows.push(groupRowNodes[gr].getAttribute("data-control"));
      }
      groups.push({
        key: groupNode.getAttribute("data-group"),
        title: textOf(groupNode, '[data-role="group-title"]'),
        unavailable: groupNode.classList.contains("unavailable"),
        rows: groupRows,
      });
    }

    // The painted detail composition, or null when no detail entry is open.
    var detailNode = doc.getElementById("detail");
    var detail = detailNode
      ? {
          surface: textOf(detailNode, '[data-role="detail-title"] .type-label'),
          subject: textOf(detailNode, '[data-role="detail-subject"]'),
          status: textOf(detailNode, '[data-role="detail-status"]'),
          rows: rowReport(detailNode.querySelectorAll(".prow")),
        }
      : null;

    var modalNode = doc.getElementById("modal-shell");
    var modal = null;
    if (modalNode) {
      var optionNodes = modalNode.querySelectorAll(".modal-option");
      var modalOptions = [];
      for (var optionIndex = 0; optionIndex < optionNodes.length; optionIndex += 1) {
        modalOptions.push({
          focusPath: optionNodes[optionIndex].getAttribute("data-focus-path"),
          focused: optionNodes[optionIndex].classList.contains("is-focused"),
          disabled: optionNodes[optionIndex].classList.contains("is-disabled"),
          label: textOf(optionNodes[optionIndex], ".modal-option-label"),
          metadata: textOf(optionNodes[optionIndex], ".modal-option-meta"),
          metadataState: optionNodes[optionIndex].getAttribute("data-metadata-state"),
          state: textOf(optionNodes[optionIndex], ".modal-option-state"),
        });
      }
      var visualizationNodes = modalNode.querySelectorAll("[data-visualization]");
      var modalVisualizations = [];
      for (var modalVisualizationIndex = 0; modalVisualizationIndex < visualizationNodes.length; modalVisualizationIndex += 1) {
        modalVisualizations.push({
          id: visualizationNodes[modalVisualizationIndex].getAttribute("data-visualization"),
          kind: visualizationNodes[modalVisualizationIndex].getAttribute("data-visualization-kind"),
          landmarkCount: visualizationNodes[modalVisualizationIndex].querySelectorAll("[data-landmark]").length,
          pairCount: visualizationNodes[modalVisualizationIndex].querySelectorAll(".waveform-shape > i").length,
          status: textOf(visualizationNodes[modalVisualizationIndex], '[data-role="waveform-status"]'),
        });
      }
      var previewNode = modalNode.querySelector('[data-role="preview-region"]');
      modal = {
        browser: modalNode.classList.contains("sample-browser"),
        title: textOf(modalNode, ".modal-title .type-heading"),
        options: modalOptions,
        visualizations: modalVisualizations,
        previewState: previewNode ? previewNode.getAttribute("data-preview-state") : null,
        previewText: previewNode ? textOf(previewNode, '[data-role="preview-playhead"]') : null,
      };
    }

    var headerNode = doc.querySelector('[data-role="strip-header"]');
    var stripHeader = headerNode
      ? {
          patchName: textOf(headerNode, '[data-role="strip-patch-name"]'),
          midiInput: textOf(headerNode, '[data-routing="patch.midiInput"]'),
          outputTrack: textOf(
            headerNode,
            '[data-routing="patch.output.outputTrack"]'
          ),
          // The header is informative: it allocates no interactive target, so
          // it carries no control identity and enters no focus order.
          controls: headerNode.querySelectorAll("[data-control]").length,
        }
      : null;
    var lifecycles = [];
    var lifecycleNodes = doc.querySelectorAll("#workspace .lifecycle");
    for (var l = 0; l < lifecycleNodes.length; l += 1) {
      lifecycles.push({
        control: lifecycleNodes[l].getAttribute("data-lifecycle-for"),
        text: lifecycleNodes[l].textContent.replace(/\s+/g, " ").trim(),
      });
    }

    var inspectorElement = doc.getElementById("inspector");
    var sends = [];
    var sendRows = inspectorElement.querySelectorAll(
      '.inspector-controls .prow[data-mixer-kind="send"]'
    );
    for (var r2 = 0; r2 < sendRows.length; r2 += 1) {
      var sendLabel = textOf(sendRows[r2], ".prow-label") || "";
      sends.push({
        trackId: Number(sendRows[r2].getAttribute("data-mixer-track")),
        bus: Number(sendRows[r2].getAttribute("data-mixer-bus")),
        label: sendLabel.replace(/^T[0-9A-Fa-f]{2}\s+/, "").toUpperCase(),
        value: textOf(sendRows[r2], ".prow-value"),
      });
    }
    var utility = rowReport(inspectorElement.querySelectorAll(".prow"));
    var routes = [];
    var routeNodes = inspectorElement.querySelectorAll("[data-route]");
    for (var routeIndex = 0; routeIndex < routeNodes.length; routeIndex += 1) {
      routes.push({
        patchId: routeNodes[routeIndex].getAttribute("data-patch-id"),
        patchName: routeNodes[routeIndex].getAttribute("data-patch-name"),
      });
    }
    var inspectorBody = inspectorElement.querySelector(
      '[data-role="inspector-controls"]'
    );
    var inspectorCorrelation = inspectorElement.querySelector(
      '[data-role="inspector-correlation"]'
    );
    var inspectorFocused = inspectorBody
      ? inspectorBody.querySelector(
          '.prow[data-state="focused"], .prow[data-state="adjusting"]'
        )
      : null;
    var inspectorFocusedVisible = null;
    if (inspectorBody && inspectorFocused) {
      var bodyRect = inspectorBody.getBoundingClientRect();
      var focusedRect = inspectorFocused.getBoundingClientRect();
      inspectorFocusedVisible =
        focusedRect.top >= bodyRect.top - 1 &&
        focusedRect.bottom <= bodyRect.bottom + 1;
    }

    // What the workspace body actually got, in the window that actually
    // shipped it. `window.innerHeight` is the page's real height, which is not
    // the authored window height — the window decoration (and, on a screen
    // exactly as tall as the window, the menu bar) takes its cut before the
    // page sees a pixel. A viewport-sized iframe does not reproduce that, and
    // a seating claim measured in one is a claim about a different surface.
    var bodyNode =
      doc.getElementById("strip") ||
      doc.getElementById("detail") ||
      doc.getElementById("modal-shell");
    var workspaceBody = null;
    if (bodyNode) {
      // The composition's own height, measured from its first child's top to
      // its last child's bottom. `scrollHeight` cannot answer this: it is
      // clamped up to the client height, so a composition that seats reports
      // the band's height rather than its own and the headroom is invisible.
      var kids = bodyNode.children;
      var contentPx = 0;
      if (kids.length > 0) {
        contentPx = Math.round(
          kids[kids.length - 1].getBoundingClientRect().bottom -
            kids[0].getBoundingClientRect().top
        );
      }
      workspaceBody = {
        id: bodyNode.id,
        contentPx: contentPx,
        bandPx: bodyNode.clientHeight,
        scrollableBy: Math.max(
          0,
          bodyNode.scrollHeight - bodyNode.clientHeight
        ),
        widthPx: Math.round(bodyNode.getBoundingClientRect().width),
      };
    }

    var focusedNode = doc.querySelector(
      "#bank .column.focused, #bank .column.correlated"
    );
    return {
      generation: model.generation,
      stateHash: model.stateHash,
      bands: bands,
      viewport: {
        widthPx: window.innerWidth,
        heightPx: window.innerHeight,
      },
      workspaceBody: workspaceBody,
      columns: columns,
      rows: rows,
      groups: groups,
      stripHeader: stripHeader,
      detail: detail,
      modal: modal,
      lifecycles: lifecycles,
      sectionAnnotation: textOf(doc, '[data-role="section-annotation"]'),
      patchIdentity: textOf(doc, '[data-role="patch-identity"]'),
      focus: {
        target: textOf(inspectorElement, '[data-role="cursor"]'),
        trackId: focusedNode
          ? Number(focusedNode.getAttribute("data-track"))
          : null,
      },
      inspector: {
        widthPx: Math.round(inspectorElement.getBoundingClientRect().width),
        cursor: textOf(inspectorElement, '[data-role="cursor"]'),
        bigReadout: textOf(inspectorElement, '[data-role="big-readout"]'),
        mute: textOf(inspectorElement, '[data-role="mute"]'),
        solo: textOf(inspectorElement, '[data-role="solo"]'),
        routes: routes,
        emptyRoute: textOf(inspectorElement, ".empty-route"),
        sends: sends,
        utility: utility,
        controlOrder: utility.map(function (row) {
          return row.control;
        }),
        focusedControl: inspectorFocused
          ? inspectorFocused.getAttribute("data-control")
          : null,
        focusedVisible: inspectorFocusedVisible,
        meter: textOf(inspectorElement, "#inspector-meter-readout"),
        hintLine: textOf(inspectorElement, '[data-role="utility-hint"]'),
        // The side region seats its entries without a scroll affordance on
        // PATCH: measured, so a relaxed `overflow` shows up as evidence
        // rather than as a CSS diff nobody reads.
        scrollableBy: Math.max(
          0,
          inspectorElement.scrollHeight - inspectorElement.clientHeight
        ),
        bodyScrollableBy: inspectorBody
          ? Math.max(0, inspectorBody.scrollHeight - inspectorBody.clientHeight)
          : 0,
        correlationHeightPx: inspectorCorrelation
          ? Math.round(inspectorCorrelation.getBoundingClientRect().height)
          : 0,
      },
      meter: textOf(doc, "#meter-readout"),
      anatomy: COLUMN_ANATOMY.slice(),
    };
  }

  // ---- listener glue (kept apart from the pure render) -------------------

  // Presentation-only meter animation state: the document on screen and the
  // latest snapshot frame. Never read by render().
  var latestModel = null;
  var latestFrame = null;

  function graphRevisionsMatch(frame, model) {
    return (
      JSON.stringify(frame.activeGraphRevision) ===
      JSON.stringify(model.status && model.status.graphRevision)
    );
  }

  function meterReading(frame, model, trackId) {
    var compatible =
      frame &&
      frame.parameterGeneration === model.generation &&
      graphRevisionsMatch(frame, model);
    var track = compatible && frame.tracks ? frame.tracks[trackId] : null;
    var rms = track ? Number(track.rms) : 0;
    if (!track || !Number.isFinite(rms) || rms < 0) {
      return { rms: 0, state: "stale" };
    }
    return { rms: rms, state: rms > 0 ? "active" : "zero" };
  }

  // Repaints all passive track meters and the selected-track numeric meter
  // from one compatible latest snapshot. Missing or incompatible values are
  // explicitly stale; compatible silence is explicitly zero.
  function updateMeter() {
    var doc = window.document;
    if (!latestModel) {
      return;
    }
    var meterNodes = doc.querySelectorAll("[data-meter-track]");
    for (var i = 0; i < meterNodes.length; i += 1) {
      var meterTrackId = Number(meterNodes[i].getAttribute("data-meter-track"));
      var reading = meterReading(latestFrame, latestModel, meterTrackId);
      meterNodes[i].style.setProperty("--meter-rms", reading.rms.toFixed(6));
      meterNodes[i].setAttribute("data-meter-state", reading.state);
      meterNodes[i].setAttribute(
        "aria-label",
        trackName(meterTrackId) +
          " meter " +
          reading.rms.toFixed(3) +
          " " +
          reading.state
      );
    }

    var trackId = focusedTrackId(latestModel);
    var caption = doc.getElementById("meter-readout");
    var inspectorMeter = doc.getElementById("inspector-meter-readout");
    if (trackId === null || !caption) {
      if (caption) {
        caption.textContent = "";
      }
      return;
    }
    var selected = meterReading(latestFrame, latestModel, trackId);
    caption.textContent = "METER " + selected.rms.toFixed(3);
    caption.setAttribute("data-meter-state", selected.state);
    caption.setAttribute(
      "aria-label",
      "selected track meter " + selected.rms.toFixed(3) + " " + selected.state
    );
    if (inspectorMeter) {
      inspectorMeter.textContent =
        "METER " + selected.rms.toFixed(3) + " / " + selected.state.toUpperCase();
      inspectorMeter.setAttribute("data-meter-state", selected.state);
    }
  }

  // Reads preview playback from exactly the same compatible latest-value
  // frame as meters. The model contributes only the canonical correlation
  // identities; the frame contributes only passive playback observation.
  // Neither object is changed during repaint.
  function updatePreviewObservation() {
    var doc = window.document;
    var region = doc.querySelector('[data-role="preview-region"]');
    if (!region || !latestModel) {
      return;
    }
    var browser = surfaceById(latestModel, "sampleBrowser");
    var summary = (browser && browser.summary) || {};
    var canonical = summary.preview || { kind: "idle" };
    var expectedIdentity = Number(summary.previewRequestId || 0);
    var expectedPatch = Number(summary.patchId);
    var compatible =
      latestFrame &&
      latestFrame.parameterGeneration === latestModel.generation &&
      graphRevisionsMatch(latestFrame, latestModel) &&
      expectedIdentity > 0 &&
      Number(latestFrame.previewIdentity) === expectedIdentity &&
      Number(latestFrame.previewPatchId) === expectedPatch;
    var playhead = compatible ? Number(latestFrame.previewPlayhead) : 0;
    var finite = Number.isFinite(playhead) && playhead >= 0 && playhead <= 1;
    var state;
    if (canonical.kind !== "playing") {
      state = String(canonical.kind || "idle");
      playhead = 0;
    } else if (!compatible || !finite) {
      state = "stale";
      playhead = 0;
    } else if (!latestFrame.previewPlaying) {
      state = "stopped";
      playhead = 0;
    } else {
      state = "playing";
    }
    region.style.setProperty("--preview-playhead", playhead.toFixed(6));
    region.setAttribute("data-preview-state", state);
    region.setAttribute(
      "aria-label",
      "preview " + state + " playhead " + playhead.toFixed(3)
    );
    var readout = region.querySelector('[data-role="preview-playhead"]');
    if (readout) {
      readout.textContent =
        state === "playing"
          ? "PLAYHEAD " + (playhead * 100).toFixed(1) + "% / PLAYING"
          : "PLAYHEAD — / " + state.toUpperCase();
    }
  }

  function observeAudio(frame) {
    latestFrame = frame;
    updateMeter();
    updatePreviewObservation();
  }

  // The paint acknowledgment for one painted document: the document's
  // semantic identity — generation, stateHash, context, active surface,
  // focus path, interaction mode — copied verbatim from the received
  // document (never invented, cached, or re-derived), plus the post-paint
  // measured viewport and per-region bounds the adapter's
  // ShellFrameObservation forwarding consumes (WP02). The region ids spell
  // ShellRegionId's serialized names so the Rust side can assemble honest
  // observations without re-deriving them.
  //
  // It also carries `strip`: how many groups the PATCH strip painted, and
  // whether it painted a flat run of rows instead of groups. Grouping is
  // decided here and nowhere else (`stripGroups`), so this is read back off
  // the painted DOM and transported — one producer. Measuring it Rust-side
  // would be a second implementation of the same rule, which is agreement
  // between copies rather than proof (mission finding F-44).
  function paintedEvidence(model) {
    var doc = window.document;
    var bands = [
      { id: "contextLine", element: "context-line" },
      { id: "identityHeader", element: "identity-header" },
      { id: "mainWorkspace", element: "workspace" },
      { id: "persistentSideRegion", element: "inspector" },
      { id: "footer", element: "footer" },
    ];
    var regions = [];
    for (var i = 0; i < bands.length; i += 1) {
      var el = doc.getElementById(bands[i].element);
      if (!el) {
        continue;
      }
      var rect = el.getBoundingClientRect();
      var label = "";
      var nodes = el.querySelectorAll("span");
      for (var n = 0; n < nodes.length; n += 1) {
        var text = nodes[n].textContent.replace(/\s+/g, " ").trim();
        if (text) {
          label = text;
          break;
        }
      }
      regions.push({
        id: bands[i].id,
        xPx: rect.x,
        yPx: rect.y,
        widthPx: rect.width,
        heightPx: rect.height,
        label: label,
      });
    }
    var strip = doc.getElementById("strip");
    var groupedRows = strip ? strip.querySelectorAll(".pgroup .prow").length : 0;
    var allRows = strip ? strip.querySelectorAll(".prow").length : 0;
    return {
      generation: model.generation,
      stateHash: model.stateHash,
      context: model.context,
      activeSurface: model.activeSurface,
      focusPath: model.focusPath,
      interactionMode: model.interactionMode,
      viewport: {
        widthPx: window.innerWidth,
        heightPx: window.innerHeight,
      },
      regions: regions,
      strip: {
        groupsPainted: strip ? strip.querySelectorAll(".pgroup").length : 0,
        // A run of painted rows with none of them inside a group: the flat
        // control run FR-001 replaced. Zero painted rows is not a flat run.
        flatControlRun: allRows > 0 && groupedRows === 0,
      },
    };
  }

  // The page half of the typed render-failure path: one small
  // JSON-serializable payload — error name, message, and the failing
  // document's semantic identity (the same generation/stateHash pair the
  // painted ack leads with) when a document is available — emitted on
  // crest://render-error, which the shell converts to the fatal typed
  // PageRenderFailed error. First error wins: the shell treats the first
  // payload as fatal, so the latch drops every later fault instead of
  // letting it replace the recorded failure. The boundary itself does no
  // rendering, no DOM access, and no formatting beyond these strings, so it
  // cannot throw before emitting.
  var renderErrorEmitted = false;

  function emitRenderError(error, model) {
    if (renderErrorEmitted) {
      return; // first error wins; the shell treats the first payload as fatal
    }
    renderErrorEmitted = true;
    var tauri = window.__TAURI__;
    if (!tauri || !tauri.event) {
      return; // headless harness: the throw already propagated to the caller
    }
    tauri.event.emit(RENDER_ERROR_EVENT, {
      name: error && error.name ? String(error.name) : "Error",
      message: error && error.message ? String(error.message) : String(error),
      generation:
        model && model.generation !== undefined ? model.generation : null,
      stateHash:
        model && model.stateHash !== undefined ? model.stateHash : null,
    });
  }

  function attachTransports() {
    var tauri = window.__TAURI__;
    if (!tauri || !tauri.event) {
      return; // headless harness drives window.crest.render directly
    }
    var projectionListener = tauri.event.listen(
      PROJECTION_EVENT,
      function (event) {
        var model = event.payload;
        latestModel = model;
        // A document that fails to render must NOT ack: the catch emits the
        // typed crest://render-error payload (fatal shell-side) and returns
        // before the acknowledgment below is ever scheduled.
        try {
          render(model);
          updateMeter();
          updatePreviewObservation();
        } catch (error) {
          emitRenderError(error, model);
          return; // a failed render must NOT ack
        }
        // Exactly one paint ack per painted document, in paint order: emitted
        // after this frame has painted, carrying the document's own identity
        // with post-paint region evidence.
        window.requestAnimationFrame(function () {
          tauri.event
            .emit(PAINTED_EVENT, paintedEvidence(model))
            .catch(function (error) {
              emitRenderError(error, model);
            });
        });
      }
    );
    var meterListener = tauri.event.listen(METER_EVENT, function (event) {
      observeAudio(event.payload);
    });
    Promise.all([projectionListener, meterListener])
      .then(function () {
        return tauri.event.emit(READY_EVENT, { ready: true });
      })
      .catch(function (error) {
        emitRenderError(error, latestModel);
      });
  }

  // Any uncaught page fault after load — including a throw inside the
  // requestAnimationFrame ack callback (paintedEvidence) — is the same typed
  // fatal condition. Both handlers share the emitRenderError first-error
  // latch with the projection listener's boundary. These are fault
  // reporters, not input capture: the page still registers no key handler.
  window.addEventListener("error", function (event) {
    emitRenderError(
      event.error || { name: "Error", message: event.message },
      latestModel
    );
  });
  window.addEventListener("unhandledrejection", function (event) {
    emitRenderError(
      event.reason instanceof Error
        ? event.reason
        : { name: "UnhandledRejection", message: String(event.reason) },
      latestModel
    );
  });

  window.addEventListener("resize", function () {
    if (latestModel) {
      revealSemanticFocus(window.document, latestModel);
    }
  });

  window.crest = {
    render: render,
    renderObservation: renderObservation,
    observeAudio: observeAudio,
  };
  attachTransports();
})();
