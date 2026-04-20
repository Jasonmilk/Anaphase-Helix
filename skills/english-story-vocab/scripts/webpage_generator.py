#!/usr/bin/env python3
"""
英语学习网页生成工具

根据词汇数据和故事内容生成可交互的HTML网页

使用示例:
    python webpage_generator.py --words words.json --story story.txt --quiz quiz.json --output learning.html
"""

import json
import argparse
import sys
from pathlib import Path
from jinja2 import Template


def load_template(template_path: Path) -> Template:
    """
    加载HTML模板
    
    Args:
        template_path: 模板文件路径
    
    Returns:
        Jinja2模板对象
    """
    with open(template_path, 'r', encoding='utf-8') as f:
        template_content = f.read()
    
    return Template(template_content)


def load_css(css_path: Path) -> str:
    """
    加载CSS样式
    
    Args:
        css_path: CSS文件路径
    
    Returns:
        CSS样式字符串
    """
    with open(css_path, 'r', encoding='utf-8') as f:
        return f.read()


def load_story(story_path: str) -> str:
    """
    加载故事内容
    
    Args:
        story_path: 故事文件路径或直接内容
    
    Returns:
        故事内容字符串
    """
    # 如果是文件路径
    if Path(story_path).exists():
        with open(story_path, 'r', encoding='utf-8') as f:
            return f.read()
    
    # 如果是直接内容
    return story_path


def load_quiz(quiz_path: str) -> list:
    """
    加载互动问答
    
    Args:
        quiz_path: 问答文件路径或JSON字符串
    
    Returns:
        问答列表
    """
    # 如果是文件路径
    if Path(quiz_path).exists():
        with open(quiz_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    # 如果是JSON字符串
    try:
        return json.loads(quiz_path)
    except:
        # 如果不是有效JSON，作为纯文本处理
        return [{'question': quiz_path, 'options': ['A', 'B', 'C', 'D'], 'answer': 0}]


def generate_html(words: list, story: str, quiz: list, css: str, output_path: str):
    """
    生成HTML网页
    
    Args:
        words: 词汇列表
        story: 故事内容
        quiz: 问答列表
        css: CSS样式
        output_path: 输出文件路径
    """
    # 准备数据
    word_list = words
    
    # 准备问答数据 - 添加索引以便在模板中使用
    quiz_list = []
    if quiz:
        for i, q in enumerate(quiz):
            q_dict = q.copy()
            q_dict['index'] = i
            # 兼容不同的字段名：将 correct 字段转换为 answer 字段
            if 'correct' in q_dict:
                q_dict['answer'] = q_dict['correct']
            quiz_list.append(q_dict)
    
    # 创建HTML模板（内嵌在脚本中，确保独立性）
    template_str = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>八卦学英语 - 有趣的词汇学习</title>
    <style>
        {{ css }}
    </style>
</head>
<body>
    <div class="container">
        <!-- 头部 -->
        <header class="header">
            <h1>🎭 八卦学英语</h1>
            <p class="subtitle">通过有趣的故事记住单词</p>
            <div class="progress-info">
                <span class="lesson-badge">Lesson {{ words|length }} Words</span>
            </div>
        </header>

        <!-- 词汇表 -->
        <section class="vocab-section">
            <h2>📚 今日词汇</h2>
            <div class="vocab-list">
                {% for word in words %}
                <div class="vocab-card" id="word-{{ word.id }}">
                    <div class="vocab-header">
                        <span class="word">{{ word.word }}</span>
                        <span class="pos">{{ word.pos }}</span>
                    </div>
                    <div class="vocab-body">
                        <p class="definition">{{ word.definition }}</p>
                        {% if word.example %}
                        <p class="example">Example: {{ word.example }}</p>
                        {% endif %}
                    </div>
                </div>
                {% endfor %}
            </div>
        </section>

        <!-- 故事区域 -->
        <section class="story-section">
            <h2>📖 八卦故事</h2>
            <div class="story-content">
                {{ story|replace('\\n', '<br>')|safe }}
            </div>
        </section>

        <!-- 互动问答 -->
        <section class="quiz-section">
            <h2>🎯 互动问答</h2>
            <div class="quiz-container">
                {% for q in quiz %}
                <div class="quiz-item" data-answer="{{ q.answer }}">
                    <p class="question">{{ q.question }}</p>
                    <div class="options">
                        {% for i in range(q.options|length) %}
                        <button class="option-btn" data-index="{{ i }}">
                            {{ q.options[i] }}
                        </button>
                        {% endfor %}
                    </div>
                    <div class="feedback"></div>
                </div>
                {% endfor %}
            </div>
        </section>

        <!-- 底部 -->
        <footer class="footer">
            <p>✨ 每天学一点，进步看得见！</p>
            <button class="save-btn" onclick="saveProgress()">💾 保存进度</button>
        </footer>
    </div>

    <script>
        // 问答交互逻辑
        document.querySelectorAll('.option-btn').forEach(btn => {
            btn.addEventListener('click', function() {
                const quizItem = this.closest('.quiz-item');
                const options = quizItem.querySelectorAll('.option-btn');
                const feedback = quizItem.querySelector('.feedback');
                const answer = parseInt(quizItem.dataset.answer);
                const selectedIndex = parseInt(this.dataset.index);

                // 禁用所有选项
                options.forEach(opt => opt.disabled = true);

                // 判断答案
                if (selectedIndex === answer) {
                    this.classList.add('correct');
                    feedback.textContent = '✅ 正确！太棒了！';
                    feedback.classList.add('correct-msg');
                    feedback.style.display = 'block';
                } else {
                    this.classList.add('wrong');
                    options[answer].classList.add('correct');
                    feedback.textContent = '❌ 再接再厉！正确答案是: ' + options[answer].textContent;
                    feedback.classList.add('wrong-msg');
                    feedback.style.display = 'block';
                }
            });
        });

        // 保存进度
        function saveProgress() {
            const progress = {
                timestamp: new Date().toISOString(),
                wordCount: {{ words|length }},
                completed: document.querySelectorAll('.quiz-item').length
            };
            
            // 在实际应用中，这里会发送到后端保存
            // 这里仅作为演示
            const blob = new Blob([JSON.stringify(progress, null, 2)], 
                {type: 'application/json'});
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'vocab_progress.json';
            a.click();
            URL.revokeObjectURL(url);
            
            alert('✓ 进度已保存！');
        }

        // 页面加载时的动画
        document.addEventListener('DOMContentLoaded', function() {
            const cards = document.querySelectorAll('.vocab-card');
            cards.forEach((card, index) => {
                card.style.opacity = '0';
                card.style.transform = 'translateY(20px)';
                setTimeout(() => {
                    card.style.transition = 'opacity 0.5s, transform 0.5s';
                    card.style.opacity = '1';
                    card.style.transform = 'translateY(0)';
                }, index * 100);
            });
        });
    </script>
</body>
</html>"""
    
    # 渲染模板
    template = Template(template_str)
    html_content = template.render(
        words=word_list,
        story=story,
        quiz=quiz_list,
        css=css
    )
    
    # 保存HTML文件
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(html_content)
    
    print(f"✓ 网页已生成: {output_path}")
    print(f"  - 词汇数: {len(words)}")
    print(f"  - 问答题数: {len(quiz_list)}")


def main():
    parser = argparse.ArgumentParser(description='英语学习网页生成工具')
    parser.add_argument('--words', required=True,
                       help='词汇JSON文件路径')
    parser.add_argument('--story', required=True,
                       help='故事内容（文件路径或直接文本）')
    parser.add_argument('--quiz', default='[]',
                       help='问答内容（文件路径或JSON字符串）')
    parser.add_argument('--css',
                       help='CSS文件路径（可选，默认使用内置样式）')
    parser.add_argument('--output', default='learning.html',
                       help='输出HTML文件路径（默认: learning.html）')
    
    args = parser.parse_args()
    
    try:
        # 加载词汇
        print("加载词汇数据...")
        with open(args.words, 'r', encoding='utf-8') as f:
            words = json.load(f)
        print(f"  - 已加载 {len(words)} 个单词")
        
        # 加载故事
        print("加载故事内容...")
        story = load_story(args.story)
        print(f"  - 故事长度: {len(story)} 字符")
        
        # 加载问答
        print("加载问答数据...")
        quiz = load_quiz(args.quiz)
        print(f"  - 问答题数: {len(quiz)}")
        
        # 加载CSS（使用内置默认样式）
        css = args.css
        if not css or not Path(css).exists():
            css = """/* 八卦学英语 - 默认样式 */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: 'Comic Sans MS', 'Chalkboard SE', sans-serif;
    background: linear-gradient(135deg, #ffecd2 0%, #fcb69f 100%);
    color: #4a4a4a;
    line-height: 1.6;
    padding: 20px;
}

.container {
    max-width: 900px;
    margin: 0 auto;
    background: rgba(255, 255, 255, 0.95);
    border-radius: 25px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.1);
    padding: 40px;
    overflow: hidden;
}

.header {
    text-align: center;
    margin-bottom: 40px;
    background: linear-gradient(90deg, #ff6b6b, #feca57);
    padding: 30px;
    border-radius: 20px;
    color: white;
    box-shadow: 0 5px 15px rgba(255, 107, 107, 0.3);
}

.header h1 {
    font-size: 2.5em;
    margin-bottom: 10px;
    text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.2);
}

.subtitle {
    font-size: 1.2em;
    opacity: 0.9;
}

.progress-info {
    margin-top: 15px;
}

.lesson-badge {
    display: inline-block;
    background: rgba(255, 255, 255, 0.3);
    padding: 8px 20px;
    border-radius: 20px;
    font-weight: bold;
}

.vocab-section, .story-section, .quiz-section {
    margin-bottom: 40px;
}

h2 {
    font-size: 1.8em;
    margin-bottom: 20px;
    color: #ff6b6b;
    border-bottom: 3px solid #feca57;
    padding-bottom: 10px;
    display: flex;
    align-items: center;
    gap: 10px;
}

.vocab-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 20px;
}

.vocab-card {
    background: linear-gradient(145deg, #fff5f5, #ffeaa7);
    border-radius: 15px;
    padding: 20px;
    border: 2px solid #ffecd2;
    transition: transform 0.3s, box-shadow 0.3s;
}

.vocab-card:hover {
    transform: translateY(-5px);
    box-shadow: 0 10px 20px rgba(255, 107, 107, 0.2);
}

.vocab-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
}

.word {
    font-size: 1.4em;
    font-weight: bold;
    color: #ff6b6b;
}

.pos {
    background: #feca57;
    color: #4a4a4a;
    padding: 4px 10px;
    border-radius: 10px;
    font-size: 0.9em;
    font-weight: bold;
}

.definition {
    margin-bottom: 10px;
    color: #2d3436;
}

.example {
    color: #636e72;
    font-style: italic;
    font-size: 0.95em;
    border-left: 3px solid #ff6b6b;
    padding-left: 10px;
}

.story-content {
    background: linear-gradient(145deg, #f8f9fa, #e9ecef);
    padding: 30px;
    border-radius: 20px;
    font-size: 1.1em;
    line-height: 1.8;
    color: #2d3436;
    border: 2px dashed #ff6b6b;
}

.quiz-item {
    background: #fff;
    border-radius: 15px;
    padding: 25px;
    margin-bottom: 20px;
    border: 2px solid #ffeaa7;
    box-shadow: 0 5px 15px rgba(0, 0, 0, 0.05);
}

.question {
    font-size: 1.2em;
    margin-bottom: 15px;
    color: #2d3436;
    font-weight: 600;
}

.options {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 15px;
}

.option-btn {
    padding: 15px;
    border: 2px solid #feca57;
    border-radius: 12px;
    background: white;
    color: #4a4a4a;
    font-size: 1em;
    cursor: pointer;
    transition: all 0.3s;
    font-family: inherit;
}

.option-btn:hover:not(:disabled) {
    background: #feca57;
    transform: scale(1.02);
}

.option-btn:disabled {
    cursor: not-allowed;
    opacity: 0.7;
}

.option-btn.correct {
    background: #00b894;
    border-color: #00b894;
    color: white;
}

.option-btn.wrong {
    background: #d63031;
    border-color: #d63031;
    color: white;
}

.feedback {
    margin-top: 15px;
    padding: 10px 15px;
    border-radius: 10px;
    font-weight: bold;
    display: none;
}

.feedback.correct-msg {
    display: block;
    background: #00b894;
    color: white;
}

.feedback.wrong-msg {
    display: block;
    background: #d63031;
    color: white;
}

.footer {
    text-align: center;
    margin-top: 40px;
    padding-top: 30px;
    border-top: 2px solid #ffeaa7;
}

.save-btn {
    background: linear-gradient(90deg, #00b894, #55efc4);
    color: white;
    border: none;
    padding: 12px 30px;
    border-radius: 25px;
    font-size: 1.1em;
    font-weight: bold;
    cursor: pointer;
    margin-top: 15px;
    transition: transform 0.3s, box-shadow 0.3s;
    font-family: inherit;
}

.save-btn:hover {
    transform: scale(1.05);
    box-shadow: 0 5px 20px rgba(0, 184, 148, 0.4);
}

@media (max-width: 768px) {
    .container {
        padding: 20px;
    }
    
    .header h1 {
        font-size: 2em;
    }
    
    .vocab-list {
        grid-template-columns: 1fr;
    }
    
    .options {
        grid-template-columns: 1fr;
    }
}"""
        
        # 生成HTML
        print("生成网页...")
        generate_html(words, story, quiz, css, args.output)
        
        print("\n✓ 网页生成完成！")
        print(f"  在浏览器中打开: {Path(args.output).absolute()}")
        
        return 0
        
    except Exception as e:
        print(f"错误: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return 1


if __name__ == '__main__':
    sys.exit(main())
