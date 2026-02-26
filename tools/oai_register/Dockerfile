FROM python:3.11-slim

# 设置无交互模式，避免 apt-get 卡住
ENV DEBIAN_FRONTEND=noninteractive

# 安装相关的依赖以及 Google Chrome
RUN apt-get update && apt-get install -y \
    wget \
    gnupg2 \
    apt-transport-https \
    ca-certificates \
    curl \
    unzip \
    && curl -fsSL https://dl.google.com/linux/linux_signing_key.pub | gpg --dearmor -o /usr/share/keyrings/google-chrome.gpg \
    && echo "deb [arch=amd64 signed-by=/usr/share/keyrings/google-chrome.gpg] http://dl.google.com/linux/chrome/deb/ stable main" > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update \
    && apt-get install -y google-chrome-stable \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制并安装依赖
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# 复制主逻辑脚本
COPY main.py .

# 设置环境变量，避免 Python 输出被缓冲
ENV PYTHONUNBUFFERED=1

# 启动脚本
CMD ["python", "main.py"]
