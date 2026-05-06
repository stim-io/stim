import type {
  ConversationToolActivity,
  MessageContent,
} from "../../controller/client";

export type ChatMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  author: string;
  sentAtLabel: string;
  content: MessageContent;
  deliveryState?: "sent" | "sending" | "failed";
  metaLabel?: string | null;
};

export type SessionSummary = {
  id: string;
  title: string;
  preview: string;
  activityLabel: string;
  unreadCount: number;
  participantLabel: string;
  messages: ChatMessage[];
  toolActivities: ConversationToolActivity[];
  live: boolean;
};
