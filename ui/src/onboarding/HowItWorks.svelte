<!--
Screen II of the room (R11): how it works, told as the same plan reached two
ways.

The two columns are the two ways to change the same plan — pressing in the
app, and changing the plan's own files — and each one is a specimen plus the
three captions that say what the picture means. This cut shows the *same*
change in both: the left column adds a component to Today by pressing Add, and
the right column is the line that press writes. An earlier cut paired today's
composer with a different record file, which made "same plan either way" a
claim the screen left to the reader. The captions *inside* the card
are the panels' whole text: they are read aloud (each specimen is `aria-hidden`
piece by piece) and they sit under the line they describe, so the picture and
its sentence move on the same beat. An earlier cut carried the same teaching as
paragraphs beside the columns; they made the screen the densest reading in the
room, and the brief for this screen is "compact".

The point the screen makes, and the line under both columns, is that neither is
the second-class door: every change has a control on a screen, and every screen
is written down in the files. That a student *can* skip the app is the thing
they cannot guess from the screens alone — so the sentence that says they never
*have* to is the one sentence in the room set in the label weight, and it lives
in the join line because both columns earn it.

The names of the two doors are deliberately plain — *In the app*, *In the
files* — and the file is named by its path, because "the JSON" is a word the
machinery already has and the student does not.

One copy for one idea: the lede names what a change *is* (a course, a date, a
topic) and stops there. The two doors are named by the columns' own titles and
"both write the same plan" is the join line's sentence, so a lede that repeated
either spent three lines saying what the screen says twice already — and left
the heading the heaviest thing on the screen the brief asks to be the most
compact (Correction 3).
-->
<script lang="ts">
  import DemoCompose from './DemoCompose.svelte';
  import DemoFiles from './DemoFiles.svelte';
</script>

<div class="ob-how">
  <div>
    <h1 class="ob-h1">Change anything, two ways.</h1>
    <p class="ob-lede">Add a course, move a date, rename a topic.</p>
  </div>

  <div class="ob-panels">
    <div class="ob-panel">
      <div>
        <p class="ob-kick">In the app</p>
        <h2 class="ob-panel__title">Press in the app, and the screen is yours.</h2>
      </div>
      <DemoCompose />
    </div>

    <div class="ob-panel">
      <div>
        <p class="ob-kick">In the files</p>
        <h2 class="ob-panel__title">Or change the line — by hand, or by AI Agent.</h2>
      </div>
      <DemoFiles />
    </div>
  </div>

  <p class="ob-join">
    Same plan either way: what you press in the app lands in the files, and what you write in
    the files shows up in the app. <b>You never have to open a file.</b>
  </p>
</div>
