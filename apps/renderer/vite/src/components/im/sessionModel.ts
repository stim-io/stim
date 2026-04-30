import type { MessageContent } from "../../controller/client";

import type { ChatMessage, SessionSummary } from "./types";

export function initialLiveMessages(): ChatMessage[] {
  return [
    createChatMessage(
      "seed-assistant",
      "assistant",
      "stim",
      "Ready",
      textContent(
        "Session drawer and messages area are now the first desktop slice. Start with a text roundtrip here.",
      ),
    ),
  ];
}

export const staticSessions: readonly SessionSummary[] = [
  {
    id: "design-sync",
    title: "Design sync",
    preview: "Session shell spacing feels close to target.",
    activityLabel: "08:42",
    unreadCount: 2,
    participantLabel: "DS",
    live: false,
    messages: [
      createChatMessage(
        "design-1",
        "assistant",
        "Nora",
        "08:39",
        textContent(
          "The desktop drawer density is getting closer to the Feishu reference.",
        ),
      ),
      createChatMessage(
        "design-2",
        "user",
        "You",
        "08:42",
        textContent(
          "Good, next we should tighten the message row spacing and unread emphasis.",
        ),
      ),
    ],
  },
  {
    id: "qa-handoff",
    title: "QA handoff",
    preview: "Mock sessions remain read-only for now.",
    activityLabel: "Yesterday",
    unreadCount: 0,
    participantLabel: "QA",
    live: false,
    messages: [
      createChatMessage(
        "qa-1",
        "assistant",
        "QA",
        "Yesterday",
        textContent(
          "Once the live controller thread is stable, we can route acceptance around it.",
        ),
      ),
    ],
  },
];

export function createChatMessage(
  id: string,
  role: ChatMessage["role"],
  author: string,
  sentAtLabel: string,
  content: MessageContent,
  options?: {
    deliveryState?: ChatMessage["deliveryState"];
    metaLabel?: string | null;
  },
): ChatMessage {
  return {
    id,
    role,
    author,
    sentAtLabel,
    content,
    deliveryState: options?.deliveryState,
    metaLabel: options?.metaLabel ?? null,
  };
}

export function textContent(text: string): MessageContent {
  return {
    parts: [{ kind: "text", text }],
    layout_hint: null,
  };
}

export function previewForContent(content: MessageContent): string {
  const preview = content.parts
    .map((part) => {
      if (part.kind === "text") {
        return part.text;
      }

      if (part.kind === "raw_html") {
        return "[html]";
      }

      return "[fragment]";
    })
    .join(" ")
    .trim();

  return preview || "No message preview";
}
