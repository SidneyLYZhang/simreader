.PHONY: all build release test lint fmt fmt-check clean run install help

CARGO := cargo
BIN_NAME := simreader

all: build test

build:
	$(CARGO) build

release:
	$(CARGO) build --release

test:
	$(CARGO) test

test-verbose:
	$(CARGO) test --verbose

lint:
	$(CARGO) clippy -- -D warnings

fmt:
	$(CARGO) fmt

fmt-check:
	$(CARGO) fmt -- --check

clean:
	$(CARGO) clean

run:
	$(CARGO) run

install:
	$(CARGO) install --path .

check:
	$(CARGO) check

help:
	@echo "可用目标:"
	@echo "  all          构建并测试 (默认)"
	@echo "  build        开发模式构建"
	@echo "  release      发布模式构建 (优化)"
	@echo "  test         运行测试"
	@echo "  test-verbose 运行测试 (详细输出)"
	@echo "  lint         运行 clippy 检查"
	@echo "  fmt          格式化代码"
	@echo "  fmt-check    检查代码格式"
	@echo "  clean        清理构建产物"
	@echo "  run          开发模式运行"
	@echo "  install      安装到系统"
	@echo "  check        快速检查编译 (不生成二进制)"
	@echo "  help         显示此帮助信息"
