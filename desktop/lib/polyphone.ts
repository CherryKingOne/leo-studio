import { pinyin } from "pinyin-pro";

/** 只有汉字需要注音，标点与字母直接放过 */
const HAN = /[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]/;

export type PolyphoneReading = {
  /** 展示与落库用的拼音，如 háng / hang */
  value: string;
  /** 轻声（无声调）候选 */
  neutral: boolean;
  /** 该字脱离语境时的常用读法，弹窗里打「默」标 */
  common: boolean;
};

export function isHanChar(text: string) {
  return HAN.test(text);
}

/**
 * 单字全部候选读音：按无声调音节分组，组内带调读音在前、轻声补在最后，
 * 于是「行」铺成 háng / hàng / hang / héng / heng / xíng / xing。
 */
export function readingCandidates(char: string): PolyphoneReading[] {
  const symbol = pinyin(char, { type: "array", multiple: true, toneType: "symbol" });
  // toneType: "num" 与 symbol 同序等长，用它的声调数字做分组键与轻声判定
  const numbered = pinyin(char, { type: "array", multiple: true, toneType: "num" });
  const common = pinyin(char, { toneType: "num" });

  const groups = new Map<string, PolyphoneReading[]>();
  symbol.forEach((value, index) => {
    const syllable = numbered[index] ?? value;
    const list = groups.get(syllable.replace(/\d$/, "")) ?? [];
    list.push({ value, neutral: /0$/.test(syllable), common: syllable === common });
    groups.set(syllable.replace(/\d$/, ""), list);
  });

  return [...groups.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([syllable, list]) =>
      // 字典已给出轻声读法（如 了 le）时不再补一个同形候选
      list.some((item) => item.neutral) ? list : [...list, { value: syllable, neutral: true, common: false }],
    );
}
