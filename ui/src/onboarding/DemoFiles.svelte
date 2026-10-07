<!--
Specimen B of screen II (R11): the same list, written down.

The block is the file, so the room marks it as the app's own developer material:
it is the one place in the room that names a path and the one place that prints a
key of the plan's own JSON. Every word in it is the app's. The path is the
`json_path` the command registry documents for `view.setComponents`
(`command_registry.rs`), with its `<name>` filled in: `today.screen` is the view
key the composer writes when it edits Today (`set_components` writes
`views.<name>.components`), the key is the one that command writes, and the four
values are the components the composer's table gives Today, in its order
(`composer/defaults.ts`) — the three that stand plus the one the catalog adds.

The beat is the whole of "the same file": a press appends the component's line,
*and the comma the line before it gains with it*, because a JSON array whose last
line carries a comma does not parse. Both arrive the way a file changes — by
being there — rather than by sliding: nothing in a file moves, it is reread. The
three lines already in the array are the screen the student had already made
their own; the array is the view's, and the app writes it whole the first time
and appends to it after, which is why this block is a crop of one key rather
than the whole file.
-->
<div class="ob-demo">
  <section class="cd-card">
    <div class="ob-file" aria-hidden="true">
      <p class="ob-file__path">content/views.json#/views/today.screen/components</p>
      <span class="ob-file__line">{ '"components": [' }</span>
      <span class="ob-file__line" data-d="1">{ '{ "surface": "today-focus" },' }</span>
      <span class="ob-file__line" data-d="1">{ '{ "surface": "today-queue" },' }</span>
      <span class="ob-file__line" data-d="1">{ '{ "surface": "coming-up" }' }<span class="ob-comma">,</span></span>
      <span class="ob-file__line ob-arrive" data-d="1">{ '{ "surface": "week-chart" }' }</span>
      <span class="ob-file__line">{ ']' }</span>
    </div>

    <div class="ob-caps">
      <span class="ob-cap" data-key="1"><b>read</b> every screen is a line in the plan's own files</span>
      <span class="ob-cap" data-key="2"><b>write</b> the app writes it when you press — and you may write it too</span>
      <span class="ob-cap" data-key="3"><b>ask</b> or have an AI Agent edit the line for you</span>
    </div>
  </section>
</div>
