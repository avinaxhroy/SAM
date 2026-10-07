<!--
Screen I of the room (R11): what it is, and what it is for.

The one screen in the app that has no plan behind it, so it is the one screen
that can only use the plan's *picture*: the artwork the app is named after,
eagle and laptop, one image, on the room's ground. Everything else on the screen
is sentences — no specimen, no diagram — because a document that shows the
machinery before saying what the machinery is for has the order wrong.

The lede and its three promises are the whole argument, and each promise is one
the rest of the app has to keep: the day's own work (Today), a date set by the
student's own grades (the review ladder), a folder of plain files they own (the
plan itself). Each promise is a plain statement with the concrete thing behind
it on the line under it — what the row does, where the next date comes from,
what is on disk — because this is the only screen where a student meets the app
before it has done anything for them. Nothing here is a claim about the build
beyond those three.
-->
<script lang="ts">
  import eagle from './eaglestudy.png';

  const promises = [
    {
      t: "Today gives you the day's work.",
      s: 'Today shows one row of work at a time: press it, run it, write down what you did.',
    },
    {
      t: 'Reviews set the next date.',
      s: 'Your grade sets it: forgot brings the topic back today, solid pushes it weeks out.',
    },
    {
      t: 'It is a folder of files you own.',
      s: 'It sits on this machine as plain files — yours to read, back up, carry on a stick, or edit by hand.',
    },
  ];
</script>

<div class="ob-hero">
  <div class="ob-say">
    <h1 class="ob-h1">Know what to study today.</h1>
    <p class="ob-lede">
      SAM keeps your courses, topics, resources and review dates in one plan. It turns that
      plan into today's work, and into the reviews that keep it learned.
    </p>
    <ul class="ob-promises">
      {#each promises as promise, i (promise.t)}
        <li class="ob-promise">
          <span class="ob-promise__n">{i + 1}</span>
          <div>
            <p class="ob-promise__t">{promise.t}</p>
            <p class="ob-promise__s">{promise.s}</p>
          </div>
        </li>
      {/each}
    </ul>
  </div>

  <figure class="ob-art">
    <img src={eagle} alt="SAM's eagle at a laptop, with a backpack, books and a mug beside it." />
  </figure>
</div>
