# `zoomies`
<p align="center">
	<img src="/assets/Cat"><br>
  Miaou Miaou!
</p>
## About

`zoomies` is a script that takes a list of NationStates regions, sorts them by update order, and informs the user when
the game API reports they have updated.

## Usage

**DO NOT RUN TWO INSTANCES OF `zoomies` AT THE SAME TIME.**

`zoomies` requires a list of regions to trigger on in `trigger_list.txt` - if this file does exist, the program will inform you to come here and figure it out.

Each trigger should be on it's own line, with no additional punctuation.

`zoomies` will prompt you for your main nation - this is used exclusively to identify the current user of the script to NS' admin.

`zoomies` will then demand you provide a poll rate. It is recommended you use a poll speed of 650ms, however you can go higher.

**While 600ms poll speeds are possible, `zoomies` is experimental. You do `zoomies` at your own peril.**

## Acknowledgments

The following people provided key contributions during the initial development process:

* `zoomies` is based on KATT by [Khronion](https://github.com/Khronion)
* [rootabeta](https://github.com/rootabeta) is the entire reason this exists

## Disclaimer

Any individual using a script-based NationStates API tool is responsible for ensuring it complies with the latest version of the [API Terms of Use](https://www.nationstates.net/pages/api.html#terms). `zoomies` is designed to comply with these rules under reasonable use conditions, but the authors are not responsible for any unintended or erroneous program behavior that breaks these rules.

Never run more than one program that uses the NationStates API at once. Doing so will likely cause your IP address to exceed the API rate limit, which will in turn cause both programs to fail.
