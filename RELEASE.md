# Release Instructions

First dry run the release like so and review changes.

    cargo smart-release bones_lib bones_bevy_asset bones_bevy_renderer quinn_runtime_bevy bones_matchmaker

Then you can generate the changelogs with

    cargo changelog --write bones_lib bones_bevy_asset bones_bevy_renderer quinn_runtime_bevy bones_matchmaker

If that looks good then just clear the changes in the working tree.

If you want to manually tweak the changelogs, go a head and do so.

Commit and push the changelogs to a PR, then merge the PR. Then pull main to get the updated changelog locally.

Dry run the release one more time and review:

    cargo smart-release bones_lib bones_bevy_asset bones_bevy_renderer quinn_runtime_bevy bones_matchmaker

> **Note:** Last time I tested this I didn't do the no-push and it got stuck because of branch protection rules, everything worked except for creating the GitHub releases and the final push. Next time we can try with either `--no-push` or just making sure to temporarily disable the branch protection rules before release.

Finally, pass the `--execute` flag to actually do the release:

    cargo smart-release bones_lib bones_bevy_asset bones_bevy_renderer quinn_runtime_bevy bones_matchmaker --execute
