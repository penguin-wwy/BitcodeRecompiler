# BitcodeRecompiler

### 构建

```
git clone https://github.com/penguin-wwy/BitcodeRecompiler.git
cd BitcodeRecompiler
cargo build
```

### 使用说明

提取Binary中bitcode，再重新编译bitcode生成Binary

目前仅支持bitcode下编译的iOS app（非fat文件），后续可能提供so的编译支持。

```
./BitcodeRecompiler --sdk sdk_path --tool ToolChain_path example
```

如不指定sdk和ToolChain路径则使用默认路径

```
/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk
/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/
```

请保持原始编译时的sdk和ToolChain版本一致。

### Build

```
git clone https://github.com/penguin-wwy/BitcodeRecompiler.git
cd BitcodeRecompiler
cargo build
```

### Use

Extract bitcode in Binary and recompile bitcode to generate Binary.

At present，only iOS app compiled under bitcode is supported which is not fat file. The next step might be to provide .so compilation support.

```
./BitcodeRecompiler --sdk sdk_path --tool ToolChain_path example
```

if not specifies sdk and ToolChain path, use the default

```
/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk
/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/
```

Keep the same SDK's version and ToolChain's version with the original compiled.