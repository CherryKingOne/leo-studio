# VoiceNovel AI 核心系统接口规范文档 (API Specification)

版本: `v1.2.0-rc`  
协议: `HTTP/2 RESTful + WebSocket`  
数据格式: `application/json`  
认证方式: `Bearer Token` 或本地无需鉴权模式（Local Desktop Mode）

---

## 目录
1. [项目与文档资产接口 (Projects & Documents)](#1-项目与文档资产接口)
2. [智能剧本解析与角色识别接口 (Script & LLM Agent)](#2-智能剧本解析与角色识别接口)
3. [角色台账与音色克隆接口 (Casting & Voice Cloning)](#3-角色台账与音色克隆接口)
4. [语音合成与局部调优接口 (TTS Synthesis & Tuning)](#4-语音合成与局部调优接口)
5. [混音编排与母带导出接口 (Audio Mixing & Master Export)](#5-混音编排与母带导出接口)
6. [WebSocket 实时流式通信协议 (Real-time Pipeline)](#6-websocket-实时流式通信协议)

---

## 1. 项目与文档资产接口

### 1.1 创建新有声书项目
- **请求方式**: `POST`
- **路径**: `/api/v1/projects`
- **请求体**:
```json
{
  "name": "长夜余火：第一卷 废土归途",
  "author": "爱潜水的乌贼",
  "genre": "科幻/废土",
  "description": "废土地图上的神秘列车与探索小队故事",
  "defaultTtsEngine": "cosyvoice-2-pro",
  "sampleRate": 44100,
  "masterLufsTarget": -14.0
}
```
- **响应体**:
```json
{
  "code": 200,
  "message": "Project created successfully",
  "data": {
    "projectId": "proj_9f8c12a4",
    "name": "长夜余火：第一卷 废土归途",
    "status": "draft",
    "createdAt": "2026-10-22T10:15:30Z",
    "chapterCount": 0
  }
}
```

### 1.2 上传电子书并自动结构化分章
- **请求方式**: `POST`
- **路径**: `/api/v1/projects/{projectId}/import-document`
- **请求格式**: `multipart/form-data`
  - `file`: 电子书文件 (`.txt`, `.epub`, `.docx`, `.pdf`)
  - `splitRegex`: 可选，自定义章节分割正则（默认内置中文章节识别正则：`第[0-9一二三四五六七八九十百千万]+[章卷回节]`）
- **响应体**:
```json
{
  "code": 200,
  "data": {
    "bookTitle": "长夜余火",
    "totalWords": 458200,
    "chapters": [
      {
        "chapterId": "chap_01",
        "order": 1,
        "title": "第一章 灰土上的列车",
        "wordCount": 3420,
        "status": "unparsed"
      },
      {
        "chapterId": "chap_02",
        "order": 2,
        "title": "第二章 夜幕惊魂",
        "wordCount": 3810,
        "status": "unparsed"
      }
    ]
  }
}
```

---

## 2. 智能剧本解析与角色识别接口

### 2.1 执行单章/整书剧本结构化拆解 (LLM Parsing)
将小说纯文本自动解析为由【旁白】与【各角色对话】构成的广播剧格式剧本。
- **请求方式**: `POST`
- **路径**: `/api/v1/projects/{projectId}/chapters/{chapterId}/parse-script`
- **请求体**:
```json
{
  "llmModel": "qwen2.5-72b-instruct",
  "detectEmotions": true,
  "detectSfxCues": true,
  "contextWindow": 10
}
```
- **响应体**:
```json
{
  "code": 200,
  "data": {
    "chapterId": "chap_01",
    "detectedCharacters": [
      {
        "name": "旁白",
        "gender": "neutral",
        "linesCount": 42
      },
      {
        "name": "商见曜",
        "gender": "male",
        "persona": "思维跳跃、冷静中带着疯狂的青年",
        "linesCount": 28
      },
      {
        "name": "蒋白棉",
        "gender": "female",
        "persona": "成熟果断、具有威严的小队组长",
        "linesCount": 24
      }
    ],
    "paragraphs": [
      {
        "paragraphId": "p_001",
        "index": 1,
        "type": "narration",
        "speakerName": "旁白",
        "text": "灰蒙蒙的天空下，狂风卷着沙砾拍打在生锈的铁轨上，一列老旧的蒸汽列车喘息着穿过废墟。",
        "emotion": "沉重肃穆",
        "emotionIntensity": 0.7,
        "pinyinFixes": [],
        "sfxCue": "远处的呜咽风声与列车铁轨咔哒声"
      },
      {
        "paragraphId": "p_002",
        "index": 2,
        "type": "dialogue",
        "speakerName": "蒋白棉",
        "text": "大家注意隐蔽，前方有灰土强盗的探照哨塔！",
        "emotion": "戒备紧张",
        "emotionIntensity": 0.85,
        "pinyinFixes": [
          { "word": "探照哨", "pinyin": "tàn zhào shào" }
        ],
        "sfxCue": "枪栓拉响的清脆金属音"
      }
    ]
  }
}
```

---

## 3. 角色台账与音色克隆接口

### 3.1 角色音色绑定与参数配置
- **请求方式**: `PUT`
- **路径**: `/api/v1/projects/{projectId}/characters/{characterId}`
- **请求体**:
```json
{
  "voiceId": "voice_cosy_warm_male_04",
  "voiceName": "沧桑质感男中音",
  "speed": 1.05,
  "pitch": 0.0,
  "style": "calm_narrative",
  "volumeDb": 0.0,
  "pauseDurationAfterMs": 400
}
```

### 3.2 零样本快速声音克隆 (Zero-Shot Clone)
上传 3~10 秒参考音频即可抽取声纹用于 TTS 推理。
- **请求方式**: `POST`
- **路径**: `/api/v1/voices/clone`
- **请求格式**: `multipart/form-data`
  - `name`: 角色音色名称（如“商见曜-专属声线”）
  - `referenceAudio`: 音频文件（wav/mp3/m4a，建议单声道44.1kHz）
  - `referenceText`: 参考音频中对应的标准文本（极大提升克隆准确度）
  - `gender`: `male` / `female`
  - `tags`: `["青年", "冷峻", "微哑"]`
- **响应体**:
```json
{
  "code": 200,
  "data": {
    "voiceId": "voice_user_cloned_8829",
    "name": "商见曜-专属声线",
    "embeddingVectorDim": 192,
    "duration": 5.4,
    "qualityScore": 0.94,
    "sampleAuditionUrl": "/api/v1/media/voices/voice_user_cloned_8829_preview.wav"
  }
}
```

---

## 4. 语音合成与局部调优接口

### 4.1 章节批量语音合成任务
- **请求方式**: `POST`
- **路径**: `/api/v1/projects/{projectId}/chapters/{chapterId}/synthesize`
- **请求体**:
```json
{
  "synthesizeScope": "all",  // "all" | "only_unrendered" | "paragraph_list"
  "paragraphIds": [],
  "ttsEngine": "cosyvoice-2-pro",
  "concurrency": 4
}
```
- **响应体**:
```json
{
  "code": 202,
  "message": "Batch synthesis job scheduled",
  "data": {
    "taskId": "task_synth_7719a",
    "chapterId": "chap_01",
    "totalParagraphs": 85,
    "estimatedSeconds": 48
  }
}
```

### 4.2 单段微调与重新生成 (In-place Regeneration)
用于局部听感不满意时的局部种子刷新或参数重绘。
- **请求方式**: `POST`
- **路径**: `/api/v1/projects/{projectId}/chapters/{chapterId}/paragraphs/{paragraphId}/regenerate`
- **请求体**:
```json
{
  "text": "大家注意隐蔽，前方有灰土强盗的探照哨塔！",
  "speakerName": "蒋白棉",
  "voiceId": "voice_female_heroic_01",
  "speed": 1.15,
  "emotion": "焦急警惕",
  "emotionIntensity": 0.9,
  "seed": 42109,
  "pinyinFixes": [
    { "index": 2, "char": "意", "pinyin": "yì" }
  ]
}
```
- **响应体**:
```json
{
  "code": 200,
  "data": {
    "paragraphId": "p_002",
    "audioUrl": "/api/v1/projects/proj_9f8c12a4/audio/p_002_v2.wav",
    "waveformUrl": "/api/v1/projects/proj_9f8c12a4/audio/p_002_v2.json",
    "durationMs": 3480
  }
}
```

---

## 5. 混音编排与母带导出接口

### 5.1 场景背景乐与音效编排 (Atmosphere Orchestration)
- **请求方式**: `POST`
- **路径**: `/api/v1/projects/{projectId}/chapters/{chapterId}/ambience`
- **请求体**:
```json
{
  "autoMatchBgm": true,
  "bgmTheme": "post_apocalyptic_mystery",
  "bgmVolumeDb": -18.0,
  "ducking": {
    "enabled": true,
    "duckingDb": -12.0,
    "attackMs": 150,
    "releaseMs": 500
  },
  "insertSfx": true
}
```

### 5.2 母带渲染与多格式导出 (Master Render & Export)
- **请求方式**: `POST`
- **路径**: `/api/v1/projects/{projectId}/export`
- **请求体**:
```json
{
  "format": "m4b",  // "m4b" | "mp3_chaptered" | "stems_zip"
  "chapterIds": ["chap_01", "chap_02"],
  "audioQuality": {
    "codec": "aac",
    "bitrate": "192k",
    "sampleRate": 44100
  },
  "metadata": {
    "title": "长夜余火：第一卷",
    "album": "长夜余火",
    "artist": "VoiceNovel AI 配音团",
    "narrator": "张震(AI仿生)",
    "coverImageUrl": "/api/v1/media/covers/cover_01.jpg",
    "includeSubtitles": true,
    "subtitleFormat": "vtt"
  },
  "stemsExport": {
    "includeNarrationTrack": true,
    "includeDialogueTrack": true,
    "includeBgmTrack": true,
    "includeSfxTrack": true
  }
}
```
- **响应体**:
```json
{
  "code": 202,
  "data": {
    "exportTaskId": "task_exp_5501",
    "estimatedTimeSeconds": 30,
    "downloadUrl": "/api/v1/downloads/proj_9f8c12a4_master.m4b"
  }
}
```

---

## 6. WebSocket 实时流式通信协议

客户端连接地址: `ws://localhost:8000/api/v1/ws/project/{projectId}`

### 6.1 服务端推送消息类型

#### (1) TTS 批量合成进度通知 (`tts_progress`)
```json
{
  "event": "tts_progress",
  "data": {
    "chapterId": "chap_01",
    "completed": 48,
    "total": 85,
    "percent": 56.4,
    "currentParagraphId": "p_049",
    "previewAudioUrl": "/temp/p_049.wav"
  }
}
```

#### (2) 音频波形与播放对齐打点 (`playback_sync`)
```json
{
  "event": "playback_sync",
  "data": {
    "chapterId": "chap_01",
    "currentTimeMs": 28450,
    "activeParagraphId": "p_012",
    "activeSpeaker": "蒋白棉"
  }
}
```

#### (3) 系统异常告警 (`error_alert`)
```json
{
  "event": "error_alert",
  "data": {
    "code": "VOICE_SAMPLE_TOO_SHORT",
    "message": "上传的声音克隆参考音频小于 3 秒，可能影响合成拟真度。"
  }
}
```
