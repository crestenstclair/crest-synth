// crest-synth projection page.
//
// PURE RENDER. `render(model)` rebuilds the five shell bands from one
// deserialized SemanticGraphicalViewModel document and nothing else: no
// Date.now, no Math.random, no accumulated state, no incidental
// iteration-order dependence (every walk follows the document's own array
// order). The same document always paints the same DOM. Both declared
// top-level contexts render here — MIXER as the sixteen-column bank and PATCH
// as its projected Overview sections — through the same shared
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

  function controlByPath(surface, path) {
    var wanted = JSON.stringify(path || null);
    var controls = (surface && surface.controls) || [];
    for (var i = 0; i < controls.length; i += 1) {
      if (JSON.stringify(controls[i].path || null) === wanted) {
        return controls[i];
      }
    }
    return null;
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
    return columnFocusedControl(column) !== null;
  }

  function columnFocusedControl(column) {
    var parameters = ["level", "pan", "mute", "solo"];
    for (var i = 0; i < parameters.length; i += 1) {
      var control = column[parameters[i]];
      if (control && control.focused) {
        return control;
      }
    }
    return null;
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
    var focusedControl = columnFocusedControl(column);
    var focused = focusedControl !== null;
    var correlated = focusedTrackId(model) === column.trackId;
    var focusedRow = columnFocusedRow(column);
    var focusPathAttribute = focusedControl
      ? ' data-focus-path="' +
        escapeHtml(JSON.stringify(focusedControl.path || null)) +
        '"'
      : "";
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
      // The separator carries no surrounding spaces so the state marks retain
      // their intrinsic single-line form as the bounded column track reflows.
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
      '"' +
      focusPathAttribute +
      ">" +
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

  // ---- shared PATCH control rows -----------------------------------------

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

  // One projected control as a subordinate or Utility row: label left, the state's
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

  function overviewHeaderHtml(model, main) {
    var summary = (main && main.summary) || {};
    var side = persistentSideSurface(model);
    var routing = ["patch.midiInput", "patch.output.outputTrack"];
    var routeText = "";
    for (var i = 0; i < routing.length; i += 1) {
      var routed = controlById(side, routing[i]);
      if (routed) {
        routeText +=
          '<span class="type-hint muted">' +
          escapeHtml(String(routed.label)) +
          " " +
          escapeHtml(controlValueText(routed)) +
          "</span>";
      }
    }
    return (
      '<header class="overview-heading" data-role="overview-heading">' +
      '<span class="type-hint patch">PATCH ' +
      escapeHtml(String(summary.patchId || UNAVAILABLE_MARK)) +
      "</span>" +
      '<span class="type-display" data-role="overview-patch-name">' +
      escapeHtml(String(summary.patchName || UNAVAILABLE_MARK)) +
      "</span>" +
      '<span class="overview-routing">' +
      routeText +
      "</span>" +
      "</header>"
    );
  }

  function overviewSummaryForControl(section, control) {
    var summaries = (section && section.controlSummaries) || [];
    var wanted = JSON.stringify((control && control.path) || null);
    for (var i = 0; i < summaries.length; i += 1) {
      if (JSON.stringify(summaries[i].controlPath || null) === wanted) {
        return summaries[i];
      }
    }
    return null;
  }

  function overviewControlHtml(control, summary, mode) {
    var state = controlState(control, mode);
    var id = controlIdOf(control);
    var focusPath = JSON.stringify(control.path || null);
    var parameterCount =
      summary && typeof summary.parameterCount === "number"
        ? summary.parameterCount
        : null;
    var summaryText =
      parameterCount === null
        ? ""
        : String(parameterCount) + (parameterCount === 1 ? " PARAM" : " PARAMS");
    var mark = stateMarkHtml(control, state);
    var hints = hintRun(control.validActions || []);
    return (
      '<article class="overview-control" data-control="' +
      escapeHtml(id) +
      '" data-focus-path="' +
      escapeHtml(focusPath) +
      '" data-state="' +
      escapeHtml(state.name) +
      '">' +
      '<div class="overview-control-label">' +
      '<span class="type-label">' +
      escapeHtml(String(control.label)) +
      "</span>" +
      (mark
        ? '<span class="type-hint" data-role="state-mark">' + mark + "</span>"
        : "") +
      "</div>" +
      '<div class="overview-control-value type-heading">' +
      escapeHtml(controlValueText(control)) +
      "</div>" +
      (summaryText
        ? '<div class="overview-control-summary type-hint muted" data-role="parameter-summary">' +
          escapeHtml(summaryText) +
          "</div>"
        : "") +
      (hints
        ? '<div class="overview-control-hints" data-role="overview-hints">' + hints + "</div>"
        : "") +
      lifecycleHtml(control, id) +
      "</article>"
    );
  }

  function patchOverviewHtml(model) {
    var main = surfaceById(model, "patchMain");
    var sections = (main && main.sections) || [];
    var mode = model.interactionMode;
    var sectionHtml = "";
    for (var sectionIndex = 0; sectionIndex < sections.length; sectionIndex += 1) {
      var section = sections[sectionIndex];
      var controlHtml = "";
      var paths = section.controlPaths || [];
      for (var pathIndex = 0; pathIndex < paths.length; pathIndex += 1) {
        var control = controlByPath(main, paths[pathIndex]);
        if (!control || !control.visible) {
          continue;
        }
        controlHtml += overviewControlHtml(
          control,
          overviewSummaryForControl(section, control),
          mode
        );
      }
      sectionHtml +=
        '<section class="overview-section" data-overview-section="' +
        escapeHtml(String(section.id)) +
        '"><div class="overview-section-heading">' +
        '<h2 class="type-label">' +
        escapeHtml(String(section.label)) +
        "</h2>" +
        '<span class="type-hint muted">' +
        String(paths.length) +
        (paths.length === 1 ? " CONTROL" : " CONTROLS") +
        "</span></div>" +
        '<div class="overview-section-controls">' +
        controlHtml +
        "</div></section>";
    }
    return (
      '<div class="patch-overview" id="patch-overview" data-role="patch-overview">' +
      overviewHeaderHtml(model, main) +
      '<div class="overview-sections">' +
      sectionHtml +
      "</div></div>"
    );
  }

  // The subordinate detail surface as the declared `CapabilityDetailShell`:
  // one composition arranging whichever capability the model names.
  //
  // It branches on no subject kind. The title is the projected value of the
  // exact Overview origin retained by the reducer's return path.
  //
  // The semantic model supplies the descriptor-authored sections and their
  // control paths. Section and row order therefore remain projected facts;
  // this shell only arranges them.
  function detailShellHtml(model) {
    var detail = surfaceByRole(model, "detail");
    var controls = (detail && detail.controls) || [];
    var mode = model.interactionMode;
    var main = surfaceById(model, "patchMain");
    var head = controlByPath(main, model.returnPath && model.returnPath.origin);
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
  // detail shell while a detail entry is open, the Overview otherwise.
  // The shell bands and the persistent side region are untouched either way.
  function patchWorkspaceHtml(model) {
    var main = surfaceById(model, "patchMain");
    var detail = surfaceByRole(model, "detail");
    var modal = surfaceByRole(model, "modal");
    var body = modal
      ? modalShellHtml(model)
      : detail
        ? detailShellHtml(model)
        : patchOverviewHtml(model);
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

  // The PATCH Utility panel follows the projected surface order. Labels,
  // identities, visibility, values, state, and actions all come from the
  // canonical document; the page owns no parallel entry registry.
  function patchUtilityHtml(model, surface, summary) {
    var mode = model.interactionMode;
    var controls = surface.controls || [];
    var rows = "";
    for (var i = 0; i < controls.length; i += 1) {
      if (!controls[i].visible) {
        continue;
      }
      rows += patchRowHtml(controls[i], mode, "panel");
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

    function rectOf(element) {
      if (!element) {
        return null;
      }
      var rect = element.getBoundingClientRect();
      return {
        xPx: Math.round(rect.x),
        yPx: Math.round(rect.y),
        widthPx: Math.round(rect.width),
        heightPx: Math.round(rect.height),
        rightPx: Math.round(rect.right),
        bottomPx: Math.round(rect.bottom),
      };
    }

    function fullyVisible(element) {
      if (!element) {
        return false;
      }
      var rect = element.getBoundingClientRect();
      return (
        rect.top >= -1 &&
        rect.left >= -1 &&
        rect.bottom <= window.innerHeight + 1 &&
        rect.right <= window.innerWidth + 1
      );
    }

    var bands = {
      contextLine: painted("context-line"),
      identityHeader: painted("identity-header"),
      workspace: painted("workspace"),
      inspector: painted("inspector"),
      footer: painted("footer"),
    };

    var computedRoot = window.getComputedStyle(doc.documentElement);
    var layoutMode = computedRoot
      .getPropertyValue("--layout-mode")
      .trim()
      .replace(/^['\"]|['\"]$/g, "");
    var regionElements = [
      ["contextLine", doc.getElementById("context-line")],
      ["identityHeader", doc.getElementById("identity-header")],
      ["workspace", doc.getElementById("workspace")],
      ["inspector", doc.getElementById("inspector")],
      ["footer", doc.getElementById("footer")],
    ];
    var regions = [];
    for (var regionIndex = 0; regionIndex < regionElements.length; regionIndex += 1) {
      regions.push({
        id: regionElements[regionIndex][0],
        bounds: rectOf(regionElements[regionIndex][1]),
      });
    }
    var visualRegionOrder = regions
      .slice()
      .sort(function (left, right) {
        return (
          left.bounds.yPx - right.bounds.yPx ||
          left.bounds.xPx - right.bounds.xPx
        );
      })
      .map(function (region) {
        return region.id;
      });

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

    // Shared subordinate/Utility rows, in painted order, with measured target
    // geometry. Overview controls have their own section-aware report below.
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
    var rows = [];

    var overviewNode = doc.getElementById("patch-overview");
    var overview = null;
    if (overviewNode) {
      var overviewSections = [];
      var overviewSectionNodes = overviewNode.querySelectorAll(
        "[data-overview-section]"
      );
      for (var overviewSectionIndex = 0; overviewSectionIndex < overviewSectionNodes.length; overviewSectionIndex += 1) {
        var overviewSectionNode = overviewSectionNodes[overviewSectionIndex];
        var overviewControls = [];
        var overviewControlNodes = overviewSectionNode.querySelectorAll(
          ".overview-control"
        );
        for (var overviewControlIndex = 0; overviewControlIndex < overviewControlNodes.length; overviewControlIndex += 1) {
          var overviewControlNode = overviewControlNodes[overviewControlIndex];
          overviewControls.push({
            control: overviewControlNode.getAttribute("data-control"),
            focusPath: overviewControlNode.getAttribute("data-focus-path"),
            state: overviewControlNode.getAttribute("data-state"),
            label: textOf(overviewControlNode, ".overview-control-label .type-label"),
            value: textOf(overviewControlNode, ".overview-control-value"),
            mark: textOf(overviewControlNode, '[data-role="state-mark"]'),
            parameterSummary: textOf(
              overviewControlNode,
              '[data-role="parameter-summary"]'
            ),
            hints: textOf(overviewControlNode, '[data-role="overview-hints"]'),
            bounds: rectOf(overviewControlNode),
            visible: fullyVisible(overviewControlNode),
          });
        }
        overviewSections.push({
          id: overviewSectionNode.getAttribute("data-overview-section"),
          label: textOf(overviewSectionNode, ".overview-section-heading h2"),
          bounds: rectOf(overviewSectionNode),
          controls: overviewControls,
        });
      }
      overview = {
        patchName: textOf(overviewNode, '[data-role="overview-patch-name"]'),
        bounds: rectOf(overviewNode),
        sections: overviewSections,
      };
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

    // What the workspace body actually got in the current containing block.
    var bodyNode =
      doc.getElementById("patch-overview") ||
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
    var semanticFocusPath = JSON.stringify(model.focusPath || null);
    var semanticFocusedNode = null;
    var semanticNodes = doc.querySelectorAll("[data-focus-path]");
    var targetSizes = [];
    for (var semanticIndex = 0; semanticIndex < semanticNodes.length; semanticIndex += 1) {
      var semanticNode = semanticNodes[semanticIndex];
      if (semanticNode.getAttribute("data-focus-path") === semanticFocusPath) {
        semanticFocusedNode = semanticNode;
      }
      targetSizes.push({
        focusPath: semanticNode.getAttribute("data-focus-path"),
        bounds: rectOf(semanticNode),
      });
    }
    var workspaceElement = doc.getElementById("workspace");
    var workspaceRect = workspaceElement.getBoundingClientRect();
    var inspectorRect = inspectorElement.getBoundingClientRect();
    var overlapWidth = Math.max(
      0,
      Math.min(workspaceRect.right, inspectorRect.right) -
        Math.max(workspaceRect.left, inspectorRect.left)
    );
    var overlapHeight = Math.max(
      0,
      Math.min(workspaceRect.bottom, inspectorRect.bottom) -
        Math.max(workspaceRect.top, inspectorRect.top)
    );
    return {
      generation: model.generation,
      stateHash: model.stateHash,
      bands: bands,
      viewport: {
        widthPx: window.innerWidth,
        heightPx: window.innerHeight,
      },
      layout: {
        mode: layoutMode,
        regions: regions,
        visualOrder: visualRegionOrder,
        workspaceInspectorOverlapPx: Math.round(overlapWidth * overlapHeight),
        horizontalOverflowPx: Math.max(
          0,
          doc.documentElement.scrollWidth - doc.documentElement.clientWidth
        ),
      },
      workspaceBody: workspaceBody,
      columns: columns,
      rows: rows,
      groups: [],
      stripHeader: null,
      overview: overview,
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
        semanticPath: semanticFocusedNode
          ? semanticFocusedNode.getAttribute("data-focus-path")
          : null,
        semanticVisible: fullyVisible(semanticFocusedNode),
        targetBounds: rectOf(semanticFocusedNode),
      },
      targetSizes: targetSizes,
      reachability: {
        workspaceScrollableBy: Math.max(
          0,
          workspaceElement.scrollHeight - workspaceElement.clientHeight
        ),
        inspectorScrollableBy: Math.max(
          0,
          inspectorElement.scrollHeight - inspectorElement.clientHeight
        ),
        focusedVisible: fullyVisible(semanticFocusedNode),
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
  // The legacy optional `strip` transport field remains absent for the
  // Overview. Detailed section, control, target, and reflow evidence is read
  // through `renderObservation`; the five canonical region measurements
  // continue through this production acknowledgment.
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
