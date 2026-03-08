# Task-025: Rollenbasierte Tool-Registrierung & Sichtbarkeit

**Epic:** epic-008
**Status:** open
**Rolle:** Developer

## Beschreibung
Tools werden nach Rolle des Agenten-Tokens gefiltert. Manager sieht andere Tools als Developer oder Tester.

## Akzeptanzkriterien
- [ ] tools/list gibt nur Tools zurück die zur Token-Rolle passen
- [ ] Manager: list_epics, create_epic, create_task, assign_task, close_epic
- [ ] Developer: get_assigned_tasks, update_task, add_log, submit_for_review
- [ ] Tester: get_review_queue, add_comment, approve_task, reject_task
- [ ] Ohne Token/Rolle: alle Tools sichtbar (Abwärtskompatibilität)
