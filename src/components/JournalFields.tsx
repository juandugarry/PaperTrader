import type { JournalContent, Side } from "../api";
import {
  formatPrice,
  formatMoney,
  moneyText,
  parseMoney,
} from "../domain/money";
export interface JournalDraft {
  thesis: string;
  entryTrigger: string;
  target: string;
  stop: string;
  plannedRisk: string;
  notes: string;
  exitReason: string;
  followedPlan: string;
  wentWell: string;
  wentPoorly: string;
  wouldChange: string;
}
export const emptyDraft = (): JournalDraft => ({
  thesis: "",
  entryTrigger: "",
  target: "",
  stop: "",
  plannedRisk: "",
  notes: "",
  exitReason: "",
  followedPlan: "",
  wentWell: "",
  wentPoorly: "",
  wouldChange: "",
});
export function contentToDraft(content: JournalContent): JournalDraft {
  return {
    thesis: content.thesis,
    entryTrigger: content.entryTrigger,
    target:
      content.targetMicros === null ? "" : moneyText(content.targetMicros),
    stop: content.stopMicros === null ? "" : moneyText(content.stopMicros),
    plannedRisk:
      content.plannedRiskMicros === null
        ? ""
        : moneyText(content.plannedRiskMicros),
    notes: content.notes,
    exitReason: content.exitReason,
    followedPlan:
      content.followedPlan === null ? "" : content.followedPlan ? "yes" : "no",
    wentWell: content.wentWell,
    wentPoorly: content.wentPoorly,
    wouldChange: content.wouldChange,
  };
}
export function draftToContent(
  draft: JournalDraft,
  side: Side,
): JournalContent {
  const price = (text: string) => {
    if (!text.trim()) return null;
    const value = parseMoney(text, 6);
    if (value <= 0)
      throw new Error(
        "Target and invalidation prices must be greater than zero.",
      );
    return value;
  };
  return {
    thesis: side === "BUY" ? draft.thesis.trim() : "",
    entryTrigger: side === "BUY" ? draft.entryTrigger.trim() : "",
    targetMicros: side === "BUY" ? price(draft.target) : null,
    stopMicros: side === "BUY" ? price(draft.stop) : null,
    plannedRiskMicros:
      side === "BUY" && draft.plannedRisk.trim()
        ? parseMoney(draft.plannedRisk)
        : null,
    notes: draft.notes.trim(),
    exitReason: side === "SELL" ? draft.exitReason.trim() : "",
    followedPlan:
      side === "SELL" && draft.followedPlan
        ? draft.followedPlan === "yes"
        : null,
    wentWell: side === "SELL" ? draft.wentWell.trim() : "",
    wentPoorly: side === "SELL" ? draft.wentPoorly.trim() : "",
    wouldChange: side === "SELL" ? draft.wouldChange.trim() : "",
  };
}
export function JournalFields({
  side,
  draft,
  onChange,
}: {
  side: Side;
  draft: JournalDraft;
  onChange: (draft: JournalDraft) => void;
}) {
  const change = (field: keyof JournalDraft, value: string) =>
    onChange({ ...draft, [field]: value });
  const text = (
    field: keyof JournalDraft,
    label: string,
    placeholder?: string,
  ) => (
    <label key={field}>
      {label}
      <textarea
        maxLength={4000}
        rows={3}
        value={draft[field]}
        onChange={(e) => change(field, e.target.value)}
        placeholder={placeholder}
      />
    </label>
  );
  return (
    <div className="journal-fields">
      {side === "BUY" ? (
        <>
          {text(
            "thesis",
            "Trading thesis",
            "Why does this trade make sense to you?",
          )}
          {text("entryTrigger", "Entry trigger", "What prompted your entry?")}
          <div className="form-row">
            <label>
              Target price (AUD)
              <input
                inputMode="decimal"
                value={draft.target}
                onChange={(e) => change("target", e.target.value)}
                placeholder="Optional"
              />
            </label>
            <label>
              Stop / invalidation price (AUD)
              <input
                inputMode="decimal"
                value={draft.stop}
                onChange={(e) => change("stop", e.target.value)}
                placeholder="Optional"
              />
            </label>
          </div>
          <label>
            Planned risk (AUD)
            <input
              inputMode="decimal"
              value={draft.plannedRisk}
              onChange={(e) => change("plannedRisk", e.target.value)}
              placeholder="Your planned maximum loss"
            />
          </label>
        </>
      ) : (
        <>
          {text(
            "exitReason",
            "Exit reason",
            "Why are you reducing or closing this position?",
          )}
          <label>
            Did you follow your original plan?
            <select
              value={draft.followedPlan}
              onChange={(e) => change("followedPlan", e.target.value)}
            >
              <option value="">Not reviewed yet</option>
              <option value="yes">Yes</option>
              <option value="no">No</option>
            </select>
          </label>
          {text("wentWell", "What went well?")}
          {text("wentPoorly", "What went poorly?")}
          {text("wouldChange", "What would you change?")}
        </>
      )}
      {text("notes", "Notes", "Optional context or follow-up")}
    </div>
  );
}
export function ContentSummary({
  side,
  content,
}: {
  side: Side;
  content: JournalContent;
}) {
  const fields =
    side === "BUY"
      ? [
          ["Thesis", content.thesis],
          ["Entry trigger", content.entryTrigger],
          [
            "Target",
            content.targetMicros === null
              ? ""
              : formatPrice(content.targetMicros),
          ],
          [
            "Stop / invalidation",
            content.stopMicros === null ? "" : formatPrice(content.stopMicros),
          ],
          [
            "Planned risk",
            content.plannedRiskMicros === null
              ? ""
              : formatMoney(content.plannedRiskMicros),
          ],
          ["Notes", content.notes],
        ]
      : [
          ["Exit reason", content.exitReason],
          [
            "Followed original plan",
            content.followedPlan === null
              ? ""
              : content.followedPlan
                ? "Yes"
                : "No",
          ],
          ["Went well", content.wentWell],
          ["Went poorly", content.wentPoorly],
          ["Would change", content.wouldChange],
          ["Notes", content.notes],
        ];
  return (
    <dl className="commentary">
      {fields.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value || "Not recorded"}</dd>
        </div>
      ))}
    </dl>
  );
}
