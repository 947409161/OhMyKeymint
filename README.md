# Oh My Keymint

[![Telegram](https://img.shields.io/static/v1?label=Telegram&message=@OhMyKeymint&color=0088cc)](https://t.me/OhMyKeymint)

Custom keystore implementation for Android Keystore Spoofer

## What is this?

This is a complete implementation of the keystore, which fully implements the AOSP AIDL interface, referencing the official AOSP implementation.

In theory, this would make it harder for detectors to identify behavior inconsistent with AOSP, thus achieving greater stealth than the FOSS branch of TrickyStore or other TrickyStore-based module like TEESimulator.

## Install and configure

**Android 12 or above required.**

1. Install this module.

2. [Configure OMK](docs/CONFIGURATION.md) if needed.

3. The active configuration and keybox files are
   `/data/adb/modules/oh_my_keymint/config/config.toml` and
   `/data/adb/modules/oh_my_keymint/config/keybox.xml`. KeyMint reads the
   keybox file at startup and persists trusted runtime keybox updates there.
Injector routing settings are compiled into the Rust source. The injector
reads caller package identities at runtime and globally includes resolved
user-installed apps across Android users and profiles. It excludes system
apps, blocks core Android identities, and rejects callers whose package
identity cannot be resolved. Google Services Framework, Google Play services,
and Play Store are hardcoded exceptions. See the
[Configuration Guide](docs/CONFIGURATION.md) for KeyMint settings and restart
requirements.

The `[main]` settings `ta_operation_delay_ms`, `ta_generation_delay_ms`, and
`ta_control_delay_ms` in `config.toml` add a fresh random wait before each
covered software TA call. Their default ranges are `[9, 21]`, `[6, 16]`, and
`[1, 4]` milliseconds, including when a field is absent. Set a range to
`[0, 0]` to disable that category. Valid changes apply to new calls without a
restart. These are extra waits, which add to the compiled injector delays;
they do not provide hardware security or guarantee a detector result. See
[TA delay ranges](docs/CONFIGURATION.md#ta-delay-ranges) for coverage and limits.

The compiled injector settings can mitigate timing-based client heuristics:
`attestation_generation_delay_ms` delays challenged key
generation replies, and `operation_start_delay_ms` delays two-way OMK
`createOperation` calls before the RPC, including calls that return business
errors. `attestation_generation_delay_ms` defaults to `[20, 30]` so that a
challenged generation stays measurably more expensive than a plain one, as it is
on a device with a real security level, while the sampled range keeps the wait
from becoming a fixed value of its own; `operation_start_delay_ms` defaults to
`0`. Both accept values up to `250` milliseconds and occupy a Binder worker while
waiting. They do not provide hardware security or guarantee a detector result.
These settings are fixed in the injector source.

## Restarting keymint and injector

The module ships two background daemons: one for `keymint`, one for `injector`.
You can restart them by following commands.

```sh
touch /data/adb/modules/oh_my_keymint/config/restart.keymint
touch /data/adb/modules/oh_my_keymint/config/restart.injector
touch /data/adb/modules/oh_my_keymint/config/restart.all
```

See the [Configuration Guide](docs/CONFIGURATION.md#how-changes-are-loaded) for
which changes need a component restart or a full device reboot.

## License

**YOU MUST AGREE TO BOTH OF THE LICENSE BEFORE USING THIS SOFTWARE.**

`AGPL-3.0-or-later`

```plaintext
OhMyKeymint - Custom keymint implementation for Android Keystore Spoofer
Copyright (C) 2025 James Clef <qwq233@qwq2333.top>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```

`Oh My Keymint License`

```plaintext
1. 您不得将本软件、本软件的任意部分或将本软件作为依赖的软件用于任何商业用途。该
   商业用途包括但不限于以盈利为目的，将本软件、本软件的任意部分或将本软件作为依
   赖的软件与其他资源、物品或服务捆绑销售。

2. 您不得暗示或明示本软件与其他软件有任何从属关系。

3. 未经本软件作者书面允许，您不得超出合理使用范围或协议许可范围使用本软件的名称。

4. 除非您所在的司法管辖区的适用法律另行规定，您同意将纠纷或争议提交至中国大陆境
   内有管辖权的人民法院管辖。

5. 本协议与GNU Affero General Public License（以下简称AGPL）共同发挥效力，
   当本协议内容与AGPL冲突时，应当优先应用本协议内容，本协议仅覆盖本软件作者拥有
   完全著作权的部分，对于使用其他协议的软件代码不发挥效力。
```

## Credit

Some code from [AOSP](https://source.android.com/)

License: `Apache-2.0`

```plaintext
Copyright 2022, The Android Open Source Project

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
