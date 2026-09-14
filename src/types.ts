export type TodoKind =
  | "assignment"
  | "turnitin"
  | "registration"
  | "assessment"
  | "other";

export interface TodoItem {
  id: string;
  title: string;
  course?: string;
  kind: TodoKind;
  deadline: string;
  url: string;
}

export interface FetchResult {
  source: "ajax" | "ical";
  items: TodoItem[];
}

export const TODO_KINDS: { kind: TodoKind; label: string; description: string }[] =
  [
    {
      kind: "assignment",
      label: "Assignments",
      description: "Coursework deadlines",
    },
    {
      kind: "turnitin",
      label: "Turnitin",
      description: "Originality reports",
    },
    {
      kind: "registration",
      label: "Registration",
      description: "Enrolment windows",
    },
    {
      kind: "assessment",
      label: "Assessments",
      description: "Quizzes and tests",
    },
    {
      kind: "other",
      label: "Other",
      description: "Questionnaires and the rest",
    },
  ];
