export type ProjectStatus = "制作中" | "母带就绪";
export type LibraryTab = "全部" | ProjectStatus | "回收站";
export type SortKey = "edited" | "progress" | "duration";

export type GenreTone = "accent" | "emerald" | "amber";

export type ProjectEntry = {
  id: string;
  title: string;
  genre: string;
  author: string;
  status: ProjectStatus;
  chapters: number;
  /** 正在制作的章节序号，用于封面进度条文案 */
  currentChapter?: number;
  /** 待选角角色数，用于封面进度条文案 */
  pendingRoles?: number;
  progress: number;
  generatedSeconds: number;
  tone: GenreTone;
  seed: number;
  editedHoursAgo: number;
  fav: boolean;
  trashed: boolean;
};

export const libraryTabs: LibraryTab[] = ["全部", "制作中", "母带就绪", "回收站"];

export const sortOptions: { key: SortKey; label: string }[] = [
  { key: "edited", label: "最近编辑" },
  { key: "progress", label: "按进度" },
  { key: "duration", label: "按已生成时长" },
];

export const initialProjects: ProjectEntry[] = [
  {
    id: "p-lord-of-mysteries",
    title: "诡秘之主：第一部 愚者",
    genre: "克苏鲁 / 西幻",
    author: "爱潜水的乌贼",
    status: "制作中",
    chapters: 12,
    currentChapter: 1,
    progress: 25,
    generatedSeconds: 858,
    tone: "accent",
    seed: 7,
    editedHoursAgo: 72,
    fav: true,
    trashed: false,
  },
  {
    id: "p-dao-gui-yi-xian",
    title: "道诡异仙：清旺来",
    genre: "东方民俗 / 悬疑",
    author: "狐尾的笔",
    status: "母带就绪",
    chapters: 1,
    progress: 100,
    generatedSeconds: 1120,
    tone: "emerald",
    seed: 23,
    editedHoursAgo: 8,
    fav: false,
    trashed: false,
  },
  {
    id: "p-long-night",
    title: "长夜余火：灰土列车",
    genre: "废土 / 科幻",
    author: "会说话的肘子",
    status: "制作中",
    chapters: 8,
    pendingRoles: 3,
    progress: 10,
    generatedSeconds: 0,
    tone: "amber",
    seed: 41,
    editedHoursAgo: 120,
    fav: false,
    trashed: false,
  },
];

export function relativeTime(hoursAgo: number) {
  if (hoursAgo < 1) return "刚刚";
  if (hoursAgo < 24) return `${Math.round(hoursAgo)} 小时前`;
  if (hoursAgo < 168) return `${Math.round(hoursAgo / 24)} 天前`;
  return `${Math.round(hoursAgo / 168)} 周前`;
}

export function durationLabel(seconds: number) {
  if (!seconds) return "待批量生成";
  return `已生成 ${Math.floor(seconds / 60)}分${String(seconds % 60).padStart(2, "0")}秒`;
}

export function coverHint(project: ProjectEntry) {
  if (project.progress === 100) return "全章混音完成";
  if (project.pendingRoles) return `${project.pendingRoles} 位角色待选角`;
  return `第 ${String(project.currentChapter ?? 1).padStart(3, "0")} 章 / 共 ${project.chapters} 章`;
}

export function projectMeta(project: ProjectEntry) {
  return [
    `${project.chapters} 章`,
    `进度 ${project.progress}%`,
    durationLabel(project.generatedSeconds),
    relativeTime(project.editedHoursAgo),
  ].join(" · ");
}

export function sortProjects(projects: ProjectEntry[], key: SortKey) {
  const value = (project: ProjectEntry) =>
    key === "edited"
      ? -project.editedHoursAgo
      : key === "progress"
        ? project.progress
        : project.generatedSeconds;
  return [...projects].sort((a, b) => value(b) - value(a));
}

/* ==================== 剧目看板：章节流水线与角色阵容 ==================== */

/** 看板通用色系：序号块、说话人标签、状态胶囊、角色头像共用 */
export type HubTone = "neutral" | "muted" | "accent" | "emerald" | "amber" | "violet";

export type ChapterState = "待拆解" | "待合成" | "合成中" | "已就绪";
export type PipelineFilter = "全部" | ChapterState;

export const pipelineFilters: { state: PipelineFilter; label: string }[] = [
  { state: "全部", label: "全部" },
  { state: "待拆解", label: "待拆解" },
  { state: "待合成", label: "待合成" },
  { state: "合成中", label: "合成中" },
  { state: "已就绪", label: "母带就绪" },
];

export type HubSpeaker = { name: string; tone: HubTone };

export type ChapterStatusIcon = "master" | "script" | "queue" | "scan" | "cast";
export type ChapterStatus = { label: string; tone: HubTone; icon: ChapterStatusIcon };

/** 行尾主动作，决定按钮形态与是否可勾选 */
export type ChapterAction =
  | { kind: "studio" }
  | { kind: "progress" }
  | { kind: "synthesize"; primary: boolean }
  | { kind: "split" }
  | { kind: "cast" };

export type HubChapter = {
  id: string;
  /** 序号块文案，如 "01" */
  no: string;
  title: string;
  /** 正文摘句；待选角一类的行放提示文案 */
  excerpt: string;
  words: number;
  /** 母带时长（秒），仅已渲染章节 */
  masterSeconds?: number;
  /** 预估时长（秒），未拆解时缺省 */
  etaSeconds?: number;
  /** 序号块配色，accent / amber 同时给整行铺底色 */
  tone: HubTone;
  state: ChapterState;
  /** 状态胶囊；合成中的行改画进度条，故缺省 */
  status?: ChapterStatus;
  percent?: number;
  /** 待拆解的章节还没有说话人，展示“待解析” */
  speakers: HubSpeaker[] | null;
  action: ChapterAction;
  /** 是否已可进入制作工作台，决定标题是链接还是纯文本 */
  openable: boolean;
  /** 折叠在“展开全部”之后 */
  collapsed?: boolean;
};

export type CastFact = {
  label: string;
  value: string;
  /** 数字类参数用等宽字体呈现 */
  mono?: boolean;
  emphasis?: "dim" | "tone";
};

export type CastLink = {
  label: string;
  /** 指向尚未移植的路由时给 href，纯反馈动作给 toast */
  href?: string;
  toast?: string;
};

export type CastMember = {
  id: string;
  name: string;
  /** 角色定位标签，如 男主角 */
  role: string;
  /** 声线气质描述 */
  trait: string;
  /** 头像文字：姓名首字 */
  initial: string;
  tone: HubTone;
  /** 试听音高（Hz），程序化发声区分不同声线 */
  hz: number;
  facts: CastFact[];
  /** 已就绪 / 待确认音色，同时供状态筛选 */
  state: "已就绪" | "待合成";
  /** 底部技术标注，如 Zero-Shot · Seed 8421；缺省时展示状态圆点 */
  footerNote?: string;
  links: CastLink[];
};

export type ProjectHub = {
  chapters: HubChapter[];
  cast: CastMember[];
};

const narrator = { name: "旁白", tone: "neutral" } as HubSpeaker;
const klein = { name: "克莱恩", tone: "accent" } as HubSpeaker;

const lordOfMysteriesHub: ProjectHub = {
  chapters: [
    {
      id: "c-001",
      no: "01",
      title: "第001章 绯红之月",
      excerpt: "痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛……",
      words: 3428,
      masterSeconds: 858,
      tone: "emerald",
      state: "已就绪",
      status: { label: "全章母带就绪", tone: "emerald", icon: "master" },
      speakers: [narrator, klein, { name: "梅丽莎", tone: "violet" }],
      action: { kind: "studio" },
      openable: true,
    },
    {
      id: "c-002",
      no: "02",
      title: "第002章 宿命之枪",
      excerpt: "左轮手枪与桌上的自杀日记，黄铜握柄还残留着体温。",
      words: 3810,
      etaSeconds: 930,
      tone: "accent",
      state: "合成中",
      percent: 62,
      speakers: [narrator, klein],
      action: { kind: "progress" },
      openable: true,
    },
    {
      id: "c-003",
      no: "03",
      title: "第003章 罗塞尔日记",
      excerpt: "穿越者前辈的隐秘手稿，字符在月光下缓慢重组。",
      words: 4120,
      etaSeconds: 1004,
      tone: "neutral",
      state: "待合成",
      status: { label: "剧本化完成 · 待合成", tone: "amber", icon: "script" },
      speakers: [narrator, klein, { name: "罗塞尔", tone: "amber" }],
      action: { kind: "synthesize", primary: true },
      openable: true,
    },
    {
      id: "c-004",
      no: "04",
      title: "第004章 灰雾之上的盛宴",
      excerpt: "长桌、烛火与不属于这个世界的低语者。",
      words: 3966,
      tone: "neutral",
      state: "待拆解",
      status: { label: "待拆解", tone: "neutral", icon: "scan" },
      speakers: null,
      action: { kind: "split" },
      openable: false,
    },
    {
      id: "c-005",
      no: "05",
      title: "第005章 占卜师的世界",
      excerpt: "黄铜镜片、灵性与一整套谨慎的自我介绍。",
      words: 3512,
      etaSeconds: 860,
      tone: "neutral",
      state: "待合成",
      status: { label: "排队中", tone: "neutral", icon: "queue" },
      speakers: [narrator, klein, { name: "邓恩", tone: "muted" }],
      action: { kind: "synthesize", primary: false },
      openable: false,
    },
    {
      id: "c-006",
      no: "06",
      title: "第006章 尊严就是相位",
      excerpt: "2 段对白未能匹配声线，合成前需完成选角。",
      words: 3704,
      etaSeconds: 908,
      tone: "amber",
      state: "待合成",
      status: { label: "待选角", tone: "amber", icon: "cast" },
      speakers: [narrator, { name: "未选角 2", tone: "amber" }],
      action: { kind: "cast" },
      openable: true,
    },
    ...([
      ["第007章 塔罗会与愚者", "长桌两端，八张面孔与一张空位。", 3288, 806],
      ["第008章 第一单生意", "灵性、黄铜镜片与讨价还价的仪式。", 3610, 888],
      ["第009章 廷根大学的清晨", "面试通知书、煤炉与一家三口的早餐。", 2940, 724],
      ["第010章 深夜的低语", "窗外的绯红月光又一次亮了。", 3455, 850],
      ["第011章 序列九·占卜家", "服魔配方与不可逆的第一步。", 4028, 982],
      ["第012章 灰雾之下的凝视", "祂始终没有离开那张长桌。", 3772, 930],
    ] as [string, string, number, number][]).map<HubChapter>(([title, excerpt, words, eta], i) => {
      const no = String(i + 7).padStart(2, "0");
      return {
        id: `c-${no}`,
        no,
        title,
        excerpt,
        words,
        etaSeconds: eta,
        tone: "neutral",
        state: "待拆解",
        status: { label: "待拆解", tone: "neutral", icon: "scan" },
        speakers: null,
        action: { kind: "split" },
        openable: false,
        collapsed: true,
      };
    }),
  ],
  cast: [
    {
      id: "cast-narrator",
      name: "旁白 (全知视角)",
      role: "主叙事",
      trait: "沉稳稳健、历史沧桑厚重感",
      initial: "白",
      tone: "neutral",
      hz: 220,
      facts: [
        { label: "绑定声线", value: "沧桑男中音 (CosyVoice-Pre)" },
        { label: "语速控制", value: "1.05x", mono: true },
        { label: "台词覆盖", value: "268 段 (64% 占比)", mono: true },
      ],
      state: "已就绪",
      links: [
        { label: "更换音色", href: "/voice-library" },
        { label: "调优", toast: "已打开声线微调面板" },
      ],
    },
    {
      id: "cast-klein",
      name: "克莱恩·莫雷蒂",
      role: "男主角",
      trait: "深思冷静、情绪激荡时带喘息颤音",
      initial: "克",
      tone: "accent",
      hz: 330,
      facts: [
        { label: "绑定声线", value: "英伦青年音 (零样本克隆)" },
        { label: "情绪基线", value: "惊恐 85% · 克制 60%", mono: true },
        { label: "台词覆盖", value: "96 段 (24% 占比)", mono: true, emphasis: "tone" },
      ],
      state: "已就绪",
      footerNote: "Zero-Shot · Seed 8421",
      links: [
        { label: "逐句精修", href: "/project/p-lord-of-mysteries/chapter/c-001" },
        { label: "调优", toast: "已打开声线微调面板" },
      ],
    },
    {
      id: "cast-melissa",
      name: "梅丽莎·莫雷蒂",
      role: "妹妹",
      trait: "清澈纯真、略带担忧的少女音",
      initial: "梅",
      tone: "violet",
      hz: 440,
      facts: [
        { label: "绑定声线", value: "清澈少女音 (Fish-Studio)" },
        { label: "语速 / 音调", value: "0.98x / +0.5st", mono: true },
        { label: "台词覆盖", value: "34 段 (8% 占比)", mono: true, emphasis: "tone" },
      ],
      state: "已就绪",
      links: [
        { label: "更换音色", href: "/voice-library" },
        { label: "调优", toast: "已打开声线微调面板" },
      ],
    },
    {
      id: "cast-benson",
      name: "班森·莫雷蒂",
      role: "兄长",
      trait: "温和持家、说话节奏偏慢",
      initial: "班",
      tone: "neutral",
      hz: 180,
      facts: [
        { label: "候选声线", value: "温厚长兄音 · 未确认" },
        { label: "出现章节", value: "第 1、2、7 章", mono: true },
        { label: "台词覆盖", value: "18 段 (4% 占比)", mono: true, emphasis: "dim" },
      ],
      state: "待合成",
      links: [
        { label: "挑声线", href: "/voice-library" },
        { label: "绑定", toast: "已把「班森·莫雷蒂」绑定到温厚长兄音" },
      ],
    },
    {
      id: "cast-roselle",
      name: "罗塞尔·古斯塔夫",
      role: "配角",
      trait: "张扬自负的冒险家腔调，尾音上扬",
      initial: "罗",
      tone: "amber",
      hz: 260,
      facts: [
        { label: "绑定声线", value: "低沉阅历音 (CosyVoice-Pre)" },
        { label: "语速控制", value: "1.00x", mono: true },
        { label: "台词覆盖", value: "22 段 (5% 占比)", mono: true, emphasis: "tone" },
      ],
      state: "已就绪",
      links: [
        { label: "更换音色", href: "/voice-library" },
        { label: "调优", toast: "已打开声线微调面板" },
      ],
    },
  ],
};

/** 已核对过的剧目看板数据，其余剧目进入时按剧目进度生成台账 */
export const projectHubs: Record<string, ProjectHub> = {
  "p-lord-of-mysteries": lordOfMysteriesHub,
};

/** 未单独核对过看板的剧目：按剧目进度生成流水线，剩余章节留在待拆解 */
function pendingHub(project: ProjectEntry): ProjectHub {
  const readyCount = Math.floor((project.chapters * project.progress) / 100);
  const narrated = readyCount === project.chapters;

  return {
    chapters: Array.from({ length: project.chapters }, (_, index): HubChapter => {
      const words = 3200 + ((project.seed * 977 + index * 613) % 900);
      const ready = index < readyCount;
      return {
        id: `c-${String(index + 1).padStart(3, "0")}`,
        no: String(index + 1).padStart(2, "0"),
        title: `第${String(index + 1).padStart(3, "0")}章`,
        excerpt: ready
          ? "全章已渲染，可进入制作工作台试听母带。"
          : "原文已入库，等待拆本引擎识别对白、旁白与情绪。",
        words,
        masterSeconds: ready ? Math.round(words / 4) : undefined,
        tone: ready ? "emerald" : "neutral",
        state: ready ? "已就绪" : "待拆解",
        status: ready
          ? { label: "全章母带就绪", tone: "emerald", icon: "master" }
          : { label: "待拆解", tone: "neutral", icon: "scan" },
        speakers: ready ? [narrator] : null,
        action: ready ? { kind: "studio" } : { kind: "split" },
        openable: ready,
        collapsed: index >= 6,
      };
    }),
    cast: [
      {
        id: "cast-narrator",
        name: "旁白",
        role: "主叙事",
        trait: narrated ? "沉稳稳健的默认叙事声线" : "拆解完成后按上下文推荐叙事声线",
        initial: "白",
        tone: "neutral",
        hz: 220,
        facts: [
          {
            label: narrated ? "绑定声线" : "候选声线",
            value: narrated ? "默认叙事音 (CosyVoice-Pre)" : "尚未绑定",
          },
          { label: "出现章节", value: `全书 ${project.chapters} 章`, mono: true },
          { label: "台词覆盖", value: "待统计", mono: true, emphasis: "dim" },
        ],
        state: narrated ? "已就绪" : "待合成",
        links: narrated
          ? [
              { label: "更换音色", href: "/voice-library" },
              { label: "调优", toast: "已打开声线微调面板" },
            ]
          : [
              { label: "挑声线", href: "/voice-library" },
              { label: "绑定", toast: "已把「旁白」绑定到默认叙事音" },
            ],
      },
    ],
  };
}

export function hubFor(project: ProjectEntry): ProjectHub {
  return projectHubs[project.id] ?? pendingHub(project);
}

/** 已就绪与合成中的行不进入批量选择 */
export function chapterIsSelectable(chapter: HubChapter) {
  return chapter.action.kind !== "studio" && chapter.action.kind !== "progress";
}

export function chapterAfterSplit(chapter: HubChapter): HubChapter {
  return {
    ...chapter,
    state: "待合成",
    tone: "neutral",
    etaSeconds: chapter.etaSeconds ?? Math.round(chapter.words / 4),
    status: { label: "剧本化完成 · 待合成", tone: "amber", icon: "script" },
    speakers: [narrator, klein],
    action: { kind: "synthesize", primary: true },
    openable: true,
  };
}

export function chapterAfterRender(chapter: HubChapter): HubChapter {
  return {
    ...chapter,
    state: "已就绪",
    tone: "emerald",
    masterSeconds: chapter.etaSeconds,
    etaSeconds: undefined,
    percent: undefined,
    status: { label: "全章母带就绪", tone: "emerald", icon: "master" },
    action: { kind: "studio" },
    openable: true,
  };
}

export function chapterRuntimeSeconds(chapter: HubChapter) {
  return chapter.masterSeconds ?? chapter.etaSeconds ?? Math.round(chapter.words / 4);
}

export function formatClock(seconds: number) {
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export function formatWords(words: number) {
  return words.toLocaleString("en-US");
}

/** header 副标题：作者 · 全书字数 · 预估全剧时长 */
export function hubSubtitle(project: ProjectEntry, chapters: HubChapter[]) {
  const words = chapters.reduce((sum, chapter) => sum + chapter.words, 0);
  const runtime = chapters.reduce((sum, chapter) => sum + chapterRuntimeSeconds(chapter), 0);
  return [
    `${project.author} 著`,
    `${(words / 10000).toFixed(1)} 万字`,
    `预估全剧 ${(runtime / 3600).toFixed(1)} 小时`,
  ].join(" · ");
}

/* ==================== 章节制作工作台 ==================== */

/** 说话人改绑下拉：option 为选项文案，label 为改绑后卡片头部显示名 */
export type CastOption = { option: string; label: string };

export type StudioLink = {
  label: string;
  toast?: string;
  /** accent 强调链接，缺省为白面子链接 */
  emphasis?: boolean;
  /** 提示条语气，缺省 info */
  tone?: "info" | "success";
};

/** 右侧配音设置面板的默认参数 */
export type StudioTuning = {
  speaker: string;
  model: string;
  /** 情绪强度 0-100 */
  emotion: number;
  /** 语速倍率 */
  speed: number;
  reverb: string;
  seed: string;
};

export type StudioParagraph = {
  id: string;
  /** 大纲与检索用的说话人短名，如 克莱恩 */
  speaker: string;
  /** 卡片头部完整显示名，如 克莱恩·莫雷蒂 */
  displayName: string;
  /** 圆点、姓名、试听按钮与卡片描边共用 */
  tone: HubTone;
  /** 情绪胶囊文案 */
  mood: string;
  /** 情绪胶囊配色，缺省沿用 tone */
  moodTone?: HubTone;
  text: string;
  /** 多音字校对标注：phrase 首字加虚线描边并注读音 */
  polyphone?: { phrase: string; reading: string };
  /** 精修中的段落允许直接改正文 */
  editable?: boolean;
  /** 段时长（秒），用于累计时间轴与大纲时长标注 */
  seconds: number;
  /** 试听特征音频率（Hz） */
  hz: number;
  /** 底部音色说明 */
  voice: string;
  /** 音色说明尾部的参数值 */
  voiceValue?: string;
  voiceValueMono?: boolean;
  links: StudioLink[];
  /** 有值则渲染说话人改绑下拉 */
  castOptions?: CastOption[];
  /** 附加技术标注，如 门外传音 · 隔音混响 */
  aside?: string;
  /** 紧随本段之后的场景音效提示 */
  cue?: { text: string; hz: number; seconds: number };
  tuning: StudioTuning;
};

/** 原著正文块，点击跳转到对应剧本卡片 */
export type StudioSourceBlock = { text: string; paragraphId: string };

export type ChapterStudio = {
  paragraphs: StudioParagraph[];
  source: StudioSourceBlock[];
  speakerOptions: string[];
  modelOptions: string[];
  reverbOptions: string[];
};

const kleinCastOptions: CastOption[] = [
  { option: "克莱恩（男主）", label: "克莱恩·莫雷蒂" },
  { option: "旁白", label: "旁白" },
  { option: "邓恩·史密斯", label: "邓恩·史密斯" },
  { option: "梅丽莎·莫雷蒂", label: "梅丽莎·莫雷蒂" },
];

const studioModelOptions = ["零样本克隆 Zero-Shot", "CosyVoice-Pre", "GPT-SoVITS 微调"];
const studioReverbOptions = ["无", "小房间", "门阻隔混响 15%", "大厅"];

const lordChapterOneStudio: ChapterStudio = {
  paragraphs: [
    {
      id: "p-001",
      speaker: "旁白",
      displayName: "旁白",
      tone: "neutral",
      mood: "沉重肃穆",
      text: "痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛，狠狠刺入了脑海深处。重叠而虚幻的低语声在耳畔疯狂回荡，窗外绯红的月光正如同粘稠的血液般，静静淌在粗糙的木制书桌上。",
      polyphone: { phrase: "重叠", reading: "chóng" },
      seconds: 8.2,
      hz: 220,
      voice: "音色 沧桑男中音 · CosyVoice-Pre · 语速",
      voiceValue: "1.05x",
      voiceValueMono: true,
      links: [
        { label: "校对多音字", toast: "多音字注音已校准", tone: "success" },
        { label: "单句重新生成", toast: "已换 Seed 重新生成该句", emphasis: true },
      ],
      cue: {
        text: "场景音效 · 粗重急促的喘息 + 书页在风中剧烈翻动 1.2s",
        hz: 160,
        seconds: 1.2,
      },
      tuning: {
        speaker: "旁白",
        model: "CosyVoice-Pre",
        emotion: 60,
        speed: 1.05,
        reverb: "无",
        seed: "3107",
      },
    },
    {
      id: "p-002",
      speaker: "克莱恩",
      displayName: "克莱恩·莫雷蒂",
      tone: "accent",
      mood: "惊恐迷茫 85%",
      moodTone: "amber",
      text: "“这是哪里……我不是在家里睡觉吗？为什么桌上会有一柄黄铜左轮手枪？！”",
      editable: true,
      seconds: 6.8,
      hz: 330,
      voice: "音色 英伦青年音 · 零样本克隆 · 语气",
      voiceValue: "颤抖气音",
      links: [
        { label: "调节情绪", toast: "已把情绪强度设为 90%", tone: "success" },
        { label: "局部重绘", toast: "正在局部重新推理，Seed 8421", emphasis: true },
      ],
      castOptions: kleinCastOptions,
      tuning: {
        speaker: "克莱恩（男主）",
        model: "零样本克隆 Zero-Shot",
        emotion: 85,
        speed: 1,
        reverb: "无",
        seed: "8421",
      },
    },
    {
      id: "p-003",
      speaker: "旁白",
      displayName: "旁白",
      tone: "neutral",
      mood: "客观平缓",
      text: "他下意识抬手抚摸自己的太阳穴，指尖传来了湿润而冰冷的黏腻感。借着绯红月光，他赫然看清了自己的双手——那上面沾满了暗红发黑的血污！",
      seconds: 11,
      hz: 220,
      voice: "音色 沧桑男中音",
      links: [{ label: "重新生成", toast: "已重新加入生成队列", emphasis: true }],
      tuning: {
        speaker: "旁白",
        model: "CosyVoice-Pre",
        emotion: 45,
        speed: 1.05,
        reverb: "无",
        seed: "5533",
      },
    },
    {
      id: "p-004",
      speaker: "梅丽莎",
      displayName: "梅丽莎·莫雷蒂",
      tone: "violet",
      mood: "轻柔担忧 60%",
      text: "“克莱恩？你还没睡吗？我听见你房间有响动，明天是廷根大学面试，可千万别睡过头了。”",
      seconds: 7.1,
      hz: 440,
      voice: "音色 清澈少女音 · 挂载门阻隔混响",
      voiceValue: "15%",
      voiceValueMono: true,
      links: [{ label: "重新生成", toast: "已重新生成该台词", emphasis: true }],
      aside: "门外传音 · 隔音混响",
      tuning: {
        speaker: "梅丽莎·莫雷蒂",
        model: "零样本克隆 Zero-Shot",
        emotion: 60,
        speed: 0.98,
        reverb: "门阻隔混响 15%",
        seed: "2264",
      },
    },
  ],
  source: [
    {
      text: "夜色深沉，克莱恩·莫雷蒂从一场怪异的梦境中惊醒。",
      paragraphId: "p-001",
    },
    {
      text: "痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛，狠狠刺入了脑海深处。",
      paragraphId: "p-001",
    },
    {
      text: "他下意识抬手抚摸自己的太阳穴，指尖传来了湿润而冰冷的黏腻感。借着绯红月光，他赫然看清了自己的双手——那上面沾满了暗红发黑的血污！",
      paragraphId: "p-003",
    },
    {
      text: "“这是哪里……我不是在家里睡觉吗？为什么桌上会有一柄黄铜左轮手枪？！”",
      paragraphId: "p-002",
    },
    {
      text: "门外传来梅丽莎轻柔的声音：“克莱恩？你还没睡吗？我听见你房间有响动，明天是廷根大学面试，可千万别睡过头了。”",
      paragraphId: "p-004",
    },
  ],
  speakerOptions: ["旁白", "克莱恩（男主）", "梅丽莎·莫雷蒂", "邓恩·史密斯"],
  modelOptions: studioModelOptions,
  reverbOptions: studioReverbOptions,
};

export const chapterStudios: Record<string, ChapterStudio> = {
  "p-lord-of-mysteries/c-001": lordChapterOneStudio,
};

/** 未逐句核对过的章节：按已识别说话人铺占位剧本，保证工作台不空转 */
function pendingStudio(project: ProjectEntry, chapter: HubChapter): ChapterStudio {
  const cast = chapter.speakers ?? [narrator];
  const paragraphs = cast.map<StudioParagraph>((tag, index) => ({
    id: `${chapter.id}-p${String(index + 1).padStart(3, "0")}`,
    speaker: tag.name,
    displayName: tag.name,
    tone: tag.tone,
    mood: tag.name === "旁白" ? "客观平缓" : "待标定",
    text:
      index === 0
        ? chapter.excerpt
        : "本章已完成剧本化拆解，逐句文案与情绪标定等待精修队列补全。",
    seconds: Math.round((7 + ((project.seed + index * 13) % 8) + 0.5) * 10) / 10,
    hz: 200 + index * 40,
    voice: "音色 待绑定声线 · CosyVoice-Pre",
    links: [{ label: "重新生成", toast: "已重新加入生成队列", emphasis: true }],
    tuning: {
      speaker: tag.name,
      model: "CosyVoice-Pre",
      emotion: 50,
      speed: 1,
      reverb: "无",
      seed: String(1000 + ((project.seed * 31 + index * 173) % 9000)),
    },
  }));

  return {
    paragraphs,
    source: [
      { text: chapter.excerpt, paragraphId: paragraphs[0].id },
      {
        text: "该章节尚未做逐句对照，拆解完成后此处会补全原著正文。",
        paragraphId: paragraphs[0].id,
      },
    ],
    speakerOptions: [...new Set(["旁白", ...cast.map((tag) => tag.name)])],
    modelOptions: studioModelOptions,
    reverbOptions: studioReverbOptions,
  };
}

export function studioFor(project: ProjectEntry, chapter: HubChapter): ChapterStudio {
  return chapterStudios[`${project.id}/${chapter.id}`] ?? pendingStudio(project, chapter);
}

/** 卡片时间轴与大纲时长标注用 MM:SS */
export function studioClock(seconds: number) {
  const whole = Math.floor(seconds);
  return `${String(Math.floor(whole / 60)).padStart(2, "0")}:${String(whole % 60).padStart(2, "0")}`;
}

/** 大纲条目的正文摘句 */
export function studioPreview(text: string) {
  return `${text.replace(/^["“]/, "").slice(0, 6)}……`;
}
