# 场景提示词生成规则

本文档详细说明场景提示词的生成规范，确保绘图指令不包含任何人物，只描述纯环境场景。

## 核心原则

**绝对禁止**：绘图指令中包含任何人物
**必须包含**：绘图指令必须是纯环境场景（empty/scenery only/no people）

## 绘图指令的组成

### 必需元素
1. **场景类型**：室内/室外
2. **空间结构**：房间、庭院、花园、厅堂等
3. **家具陈设**：桌椅、床榻、柜子、屏风等
4. **装饰物品**：窗帘、花瓶、画作、灯笼、饰品等
5. **建筑元素**：门窗、柱子、屋顶、墙壁等
6. **自然元素**：树木、花草、天空、月亮、水面等
7. **光线效果**：烛光、月光、阳光、阴影等
8. **质感细节**：丝绸、木头、石头、布料等
9. **环境氛围**：喜庆、阴郁、宁静、压抑等
10. **技术参数**：分辨率、风格、宽高比等

### 强制关键词
绘图指令必须包含以下至少一个关键词：
- `empty`
- `scenery only`
- `no people`
- `unoccupied`
- `abandoned`（废弃场景）
- `deserted`（荒凉场景）

## 禁止内容清单

### 任何人物相关词汇
- ❌ 人物：man, woman, person, character, figure, human, people, woman, man, girl, boy, lady, gentleman, master, servant, maid
- ❌ 人物部位：face, hand, body, head, foot, arm, leg, eye, ear, mouth, nose
- ❌ 人物动作：sitting, standing, walking, running, lying, sleeping, eating, drinking, talking, crying, laughing
- ❌ 人物状态：tired, happy, sad, angry, fearful, anxious, worried
- ❌ 人物外观：beautiful, ugly, young, old, tall, short, thin, fat
- ❌ 人物衣物：dress, robe, coat, shirt, pants, shoes, hat, jewelry, accessory
- ❌ 人物配饰：ring, necklace, bracelet, earring, hairpin
- ❌ 人物暗示：silhouette, shadow of person, human figure in distance, someone's belongings
- ❌ 人物互动：couple, family, group, crowd, meeting, talking, conversation

### 常见错误示例
```
❌ 错误：A woman standing in the room
✅ 正确：An empty room with furniture

❌ 错误：Interior scene, someone is sitting on the chair
✅ 正确：Interior scene, a chair in the center

❌ 错误：Bedroom with clothes on the bed
✅ 正确：Bedroom with a neatly made bed

❌ 错误：A man's shoes on the floor
✅ 正确：Empty room, wooden floor

❌ 错误：A woman's shadow on the wall
✅ 正确：Shadows from candlelight on the wall
```

## 允许内容清单

### 空间结构
- ✅ `interior of a room`
- ✅ `exterior of a building`
- ✅ `ancient Chinese hall`
- ✅ `traditional garden`
- ✅ `narrow side room`
- ✅ `spacious courtyard`

### 家具陈设
- ✅ `wooden table with carved patterns`
- ✅ `ornate canopy bed`
- ✅ `rosewood chair with cushions`
- ✅ `folding screen with landscape painting`
- ✅ `cabinet with drawers`
- ✅ `stool near the window`

### 装饰物品
- ✅ `red silk curtains`
- ✅ `porcelain vase on the table`
- ✅ `lantern hanging from the ceiling`
- ✅ `scroll painting on the wall`
- ✅ `incense burner releasing smoke`
- ✅ `decorative pillow`

### 建筑元素
- ✅ `wooden door carved with patterns`
- ✅ `paper window with wooden lattice`
- ✅ `stone pillar with dragon relief`
- ✅ `tiled roof with eaves`
- ✅ `brick wall covered with ivy`

### 自然元素
- ✅ `willow tree by the pond`
- ✅ `lotus flowers in bloom`
- ✅ `moonlight filtering through leaves`
- ✅ `still water reflecting the moon`
- ✅ `fallen leaves on the ground`
- ✅ `withered flowers`

### 光线效果
- ✅ `flickering candlelight`
- ✅ `soft morning light`
- ✅ `dim lamp light`
- ✅ `moonlight casting shadows`
- ✅ `sunlight streaming through window`
- ✅ `warm glow from the fireplace`

### 质感细节
- ✅ `smooth silk fabric`
- ✅ `rough wood grain`
- ✅ `cold stone surface`
- ✅ `delicate embroidery`
- ✅ `polished brass`
- ✅ `faded wallpaper`

### 环境氛围
- ✅ `festive and joyful`
- ✅ `gloomy and oppressive`
- ✅ `peaceful and serene`
- ✅ `mysterious and eerie`
- ✅ `nostalgic and melancholic`
- ✅ `cold and desolate`

## 画面描述 vs 绘图指令

### 区别说明
- **画面描述**：可以提及故事背景、人物活动、情节发展（中文）
- **绘图指令**：必须只描述纯环境，无任何人物（英文）

### 示例对比

#### 示例1：婚房场景

**画面描述**（可以提及人物）：
```
展现大婚当夜极度喜庆却又诡异的氛围。满屋的大红绸缎与"囍"字，由于陆砚礼的剧痛挣扎，桌上的合卺酒杯翻倒，床铺凌乱，龙凤烛火在风中摇曳，投射出狰狞的长影。
```

**绘图指令**（必须无人物）：
```
Interior of a luxurious ancient Chinese wedding chamber, empty. Saturated red silk curtains and "Double Happiness" banners. Flickering dragon-and-phoenix candles casting long, dancing shadows. On the rosewood table, an overturned wine cup and scattered red dates. A large ornate canopy bed with messy red silk bedding. The atmosphere is festive yet suffocating and eerie. Cinematic lighting, 8k, hyper-realistic. --ar 16:9 --v 6.0
```

#### 示例2：回忆场景

**画面描述**（可以提及情节）：
```
回忆中的沈府花园，石亭坐落在荷花池畔，垂柳依依。石桌上放着一个空掉的精致锦盒，暗示原本装在那里的暖玉已被强行抢走。阳光柔和但带着一丝忧伤的滤镜感。
```

**绘图指令**（必须无人物）：
```
A serene traditional Chinese garden with a stone pavilion by a lotus pond, empty. Soft daylight filtering through weeping willow branches. On the stone table in the pavilion, a small decorative box is left open. The water is still, and the atmosphere is nostalgic and slightly melancholic. High-end ancient garden design, 8k, soft focus background. --ar 16:9 --v 6.0
```

## 典型场景模板

### 室内场景模板

```
Interior of a [空间类型], empty.
[家具陈设描述]
[装饰物品描述]
[光线效果描述]
[质感细节描述]
[环境氛围描述]
[技术参数] --ar 16:9 --v 6.0
```

**示例**：
```
Interior of a traditional Chinese study room, empty. A large desk in the center with ink stone and brushes. Bookshelves filled with ancient scrolls on both walls. A paper lamp hanging from the ceiling casting warm light. A wooden floor with tatami mats. The atmosphere is scholarly and peaceful. 8k, traditional Chinese aesthetics. --ar 16:9 --v 6.0
```

### 室外场景模板

```
[场景类型] in [地点], empty.
[自然元素描述]
[建筑元素描述]
[光线效果描述]
[环境氛围描述]
[技术参数] --ar 16:9 --v 6.0
```

**示例**：
```
A traditional Chinese courtyard in autumn, empty. Red maple leaves falling on the stone ground. A small pavilion in the center with a round table. Trees with golden leaves surrounding the courtyard. Soft afternoon sunlight casting long shadows. The atmosphere is nostalgic and peaceful. 8k, hyper-realistic foliage. --ar 16:9 --v 6.0
```

### 回忆场景模板

```
[场景描述], empty.
[时光滤镜描述]
[关键物品描述]
[环境细节描述]
[怀旧氛围描述]
[技术参数] --ar 16:9 --v 6.0
```

**示例**：
```
A childhood bedroom, empty. Soft nostalgic lighting with warm tones. A small wooden bed with a teddy bear. Toys scattered on the floor. A window showing a sunny day outside. The atmosphere is dreamy and full of memories. 8k, soft focus. --ar 16:9 --v 6.0
```

## 检查清单

生成场景提示词后，必须检查：

- [ ] 绘图指令是否包含关键词：empty / scenery only / no people / unoccupied
- [ ] 是否没有任何人物词汇：man, woman, person, character, figure等
- [ ] 是否没有任何人物部位：face, hand, body等
- [ ] 是否没有任何人物动作：sitting, standing, walking等
- [ ] 是否没有任何人物衣物：dress, robe, coat等
- [ ] 是否没有任何人物暗示：silhouette, shadow of person等
- [ ] 是否描述了场景的空间结构
- [ ] 是否描述了家具陈设和装饰物品
- [ ] 是否描述了光线效果和质感细节
- [ ] 是否描述了环境氛围
- [ ] 是否包含了技术参数（8k, --ar 16:9, --v 6.0）

## 总结

生成场景提示词时，记住以下核心要点：

1. **画面描述**可以提及人物和故事背景（中文）
2. **绘图指令**必须只描述纯环境，无任何人物（英文）
3. 必须包含强制关键词：empty / scenery only / no people
4. 详细描述场景的空间、家具、装饰、光线、氛围
5. 严禁任何人物相关词汇
6. 检查清单必须全部通过

遵循以上规则，可以生成高质量的场景提示词，确保AI生成的图片是纯净的环境场景，不包含任何人物。
