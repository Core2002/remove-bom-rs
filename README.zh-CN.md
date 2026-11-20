# remove-bom

[English](README.md) | **简体中文**

一个小巧的命令行工具（同时也是一个极小的库），递归地去除文件开头的 UTF-8 BOM。

文件开头出现 UTF-8 BOM（`EF BB BF`）并不违反规范，但它经常让"只看文件前几个字节"的东西出问题：
脚本的 shebang 行、CSV 表头、Makefile，以及各种解析器。`remove-bom` 会遍历目录树，把你关心的
文件里那段前缀去掉。

## 特性

- **递归** — 从当前目录开始，遍历其下所有子目录。
- **精准** — 只处理扩展名匹配的文件（`-e txt,rs`）。
- **幂等** — 没有 BOM 的文件一个字节都不会被改动。
- **默认安全** — 改写经由同目录下的临时文件加一次原子 `rename` 完成；权限会被保留，失败时
  不会留下写了一半的文件。
- **诚实报错** — I/O 问题按文件报告到 stderr，其余文件照常处理，退出状态如实反映是否有失败。
- **也提供库** — `remove_bom::remove_bom(path)` 处理单个文件。

## 安装

需要 Rust 1.85 或更高版本（本 crate 使用 `edition = "2024"`）。

```bash
git clone https://github.com/Core2002/remove-bom-rs
cd remove-bom-rs

# 安装到 ~/.cargo/bin
cargo install --path .

# ……或者只构建
cargo build --release   # 可执行文件在 target/release/remove-bom
```

> crates.io 上的 `remove-bom` 是 2018 年发布的原始 0.1.0 版本。`cargo install remove-bom`
> 装到的是那个旧快照，而不是本仓库的代码——要使用当前代码请按上面的方式从源码构建。

## 用法

```text
remove-bom --exts <extensions>...
```

| 选项 | 说明 |
| --- | --- |
| `-e, --exts <extensions>...` | **必填。** 文件扩展名，可用逗号分隔（`-e txt,rs`），也可以重复给出（`-e txt -e rs`）。开头的点可省略：`txt` 与 `.txt` 等价。 |
| `-h, --help` | 打印帮助。 |
| `-V, --version` | 打印版本。 |

示例：

```bash
# 去掉当前目录下所有 .txt 和 .rs 文件的 BOM
remove-bom -e txt,rs

# 同上，长选项写法
remove-bom --exts=html,css,js

# 重复给出选项同样可用
remove-bom -e .txt -e .rs
```

一次实际运行：

```console
$ printf '\xef\xbb\xbfhello\n' > a.txt
$ remove-bom -e txt
./a.txt
$ od -An -tx1 a.txt
 68 65 6c 6c 6f 0a
```

当所有被访问的文件都处理成功时，`remove-bom` 以状态码 `0` 退出；只要有一个文件或目录处理失败，
就以 `1` 退出。失败信息逐条打印到 stderr，且不会让遍历提前中断。

## 每个文件被做了什么

1. 目录一律保留，以便继续向下遍历；文件名以某个列出的扩展名结尾（即 `.<扩展名>`）时才会被选中。
   这是字面量、大小写敏感的后缀匹配，所以 `tar.gz` 这种多段扩展名同样可用。
2. 读取前三个字节。如果不是 `EF BB BF`，文件保持原样——但它仍会被打印出来，表示它被访问过。
3. 否则把剩余字节复制到同目录下一个唯一命名的临时文件（`.NAME.remove-bom-<pid>-<n>.tmp`），
   把原文件的权限复制过去，再用 `rename` 把临时文件覆盖到原位置。`rename` 是原子的，因此文件
   永远不会以写了一半的状态被看到。中途任何一步失败，临时文件都会被删除，原文件保持完好。

临时文件不会使用 `<file>.tmp` 这类名字，所以恰好叫这个名字的无关文件是安全的。

## 行为与限制

在你把工具指向重要目录之前，值得先了解这些：

- 它始终作用于**当前工作目录**，没有路径参数。请先 `cd`。
- 扩展名匹配**大小写敏感**：`-e txt` 不会匹配 `README.TXT`。
- 扩展名被当作**字面文本**，绝不会当作模式：`-e 'txt|rs'` 只会查找以 `.txt|rs` 结尾的文件，
  什么也匹配不到。
- 如果扩展名列表在去空白后为空，会直接报错，而不是静默地什么都不做。
- **没有忽略规则。** 隐藏文件、`.git/`、`target/` 之类的目录同样会被遍历。
- 不跟随符号链接，指向文件的符号链接也不会被改写。
- 没有 BOM 的文件**会被打印但不会被修改**——输出是"访问过的文件列表"，而非"改动过的文件列表"。
- 改写是**原地**进行的：没有 dry-run 模式，也没有备份。文件会被替换成新的 inode，因此硬链接会
  断开，扩展属性/ACL 不会被继承（权限会保留）。
- 改写过程中崩溃或收到 `SIGKILL`，可能残留一个隐藏的 `.NAME.remove-bom-<pid>-<n>.tmp` 文件；
  此时原文件是完好的，残留文件可以直接删除。
- I/O 错误（权限不足、目录不可读等）会打印到 stderr 并导致退出状态为 `1`，其余文件仍会继续处理。
- 文件名不是合法 UTF-8 时，会通过有损解码参与匹配，因此文件名以纯 ASCII 扩展名结尾的文件仍能
  被正常处理。

鉴于以上几点，建议在干净的工作区里运行，跑完再 `git diff` 看一眼——BOM 的移除会表现为每个受
影响文件顶部的一行改动。

## 作为库使用

```rust
use remove_bom::remove_bom;

fn main() -> std::io::Result<()> {
    // Ok(true)  => 找到了 BOM 并已去除
    // Ok(false) => 文件开头没有 BOM，未被改动
    if remove_bom("a.txt")? {
        println!("stripped the BOM from a.txt");
    }
    Ok(())
}
```

`remove_bom` 接受任何实现了 `AsRef<Path>` 的参数。自 0.2.0 起它返回 `io::Result<bool>`，不再
panic，也不再返回裸 `bool`；出错时（文件不存在、权限不足等）临时文件已被清理，原文件保持原样。

## 开发

```bash
cargo build
cargo build --release
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test          # 9 个单元测试，覆盖改写流程及其失败路径
```

## 许可证

BSD-2-Clause — 见 [LICENSE](LICENSE)。Copyright (c) 2018, hyq。

## 致谢

最初由 [hyq](https://github.com/hyqhyq3) 以
[hyqhyq3/remove-bom-rs](https://github.com/hyqhyq3/remove-bom-rs) 之名编写；本仓库让它在当前
Rust 工具链上保持可构建（edition 2024、依赖更新、`rustfmt` 格式化），并修复了错误处理与改写
路径的问题。
