/// Tests für CLI v0.3.0 – Attachment-Subcommands (Tasks b6bcf490 + 2a91d0cd)
///
/// Prüft:
/// 1. cmd_attach() existiert und ruft multipart POST /api/.../attachments auf
/// 2. cmd_attach() leitet MIME-Type aus Dateiendung ab
/// 3. cmd_attach() zeigt Fortschrittsanzeige bei Dateien > 1 MB
/// 4. cmd_attach() validiert Argumente (project/task + Dateipfad pflicht)
/// 5. cmd_attachments() existiert und ruft GET /api/.../attachments auf
/// 6. cmd_attachments() gibt tabellarische Ausgabe aus
/// 7. cmd_download() existiert und ruft GET /api/.../attachments/:id auf
/// 8. cmd_download() unterstützt optionalen Output-Pfad
/// 9. Alle drei Subcommands sind im main case-Dispatch registriert
/// 10. VERSION wird auf 0.3.0 erhöht
#[cfg(test)]
mod tests {
    use crate::controllers::cli_controller::build_cli_script;

    fn script() -> String {
        build_cli_script("http://localhost:3000")
    }

    /// Hilfsfunktion: extrahiert den Bash-Funktionskörper einer benannten Funktion
    fn extract_fn(script: &str, fn_name: &str) -> String {
        let marker = format!("{fn_name}() {{");
        if let Some(start) = script.find(&marker) {
            let body = &script[start..];
            let mut depth = 0usize;
            let mut result = String::new();
            for ch in body.chars() {
                result.push(ch);
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
            }
            result
        } else {
            String::new()
        }
    }

    // ─── cmd_attach Tests ────────────────────────────────────────────────────

    /// cmd_attach existiert und sendet multipart POST an /attachments
    #[test]
    fn test_cmd_attach_exists_and_calls_upload_api() {
        let s = script();
        let f = extract_fn(&s, "cmd_attach");
        assert!(
            !f.is_empty(),
            "cmd_attach() muss im CLI-Script vorhanden sein"
        );
        assert!(
            f.contains("/attachments"),
            "cmd_attach muss POST /api/projects/:id/tasks/:task_id/attachments aufrufen, gefunden:\n{f}"
        );
        // Multipart-Upload benötigt -F oder --form Flag
        assert!(
            f.contains("-F ") || f.contains("--form ") || f.contains("multipart"),
            "cmd_attach muss curl -F für multipart/form-data Upload verwenden, gefunden:\n{f}"
        );
    }

    /// cmd_attach validiert Argumente (project/task-slug + Dateipfad pflicht)
    #[test]
    fn test_cmd_attach_validates_args() {
        let s = script();
        let f = extract_fn(&s, "cmd_attach");
        assert!(
            f.contains("Usage") || f.contains("usage"),
            "cmd_attach muss eine Usage-Hilfe ausgeben wenn Argumente fehlen:\n{f}"
        );
        assert!(
            f.contains("exit 1"),
            "cmd_attach muss mit exit 1 enden wenn Argumente fehlen:\n{f}"
        );
    }

    /// cmd_attach unterstützt --name Flag für Anzeigename
    #[test]
    fn test_cmd_attach_supports_name_flag() {
        let s = script();
        let f = extract_fn(&s, "cmd_attach");
        assert!(
            f.contains("--name"),
            "cmd_attach muss --name Flag für optionalen Anzeigenamen unterstützen, gefunden:\n{f}"
        );
    }

    /// cmd_attach leitet MIME-Type aus Dateiendung ab
    #[test]
    fn test_cmd_attach_derives_mime_type() {
        let s = script();
        let f = extract_fn(&s, "cmd_attach");
        // MIME-Type-Ableitung: entweder via file/mimetype-Kommando oder via case-Statement
        let has_mime_logic = f.contains("mime")
            || f.contains("MIME")
            || f.contains("content_type")
            || f.contains("Content-Type")
            || f.contains(".pdf")
            || f.contains("octet-stream");
        assert!(
            has_mime_logic,
            "cmd_attach muss MIME-Type aus Dateiendung ableiten (fallback: application/octet-stream), gefunden:\n{f}"
        );
    }

    /// cmd_attach zeigt Fortschrittsanzeige für große Dateien
    #[test]
    fn test_cmd_attach_shows_progress_for_large_files() {
        let s = script();
        let f = extract_fn(&s, "cmd_attach");
        // Fortschritt bei > 1MB: entweder via curl --progress-bar oder manuelle Anzeige
        let has_progress = f.contains("progress")
            || f.contains("1048576")
            || f.contains("1MB")
            || f.contains("1 MB")
            || f.contains("--progress-bar")
            || f.contains("-#");
        assert!(
            has_progress,
            "cmd_attach muss Fortschrittsanzeige bei Dateien > 1MB zeigen, gefunden:\n{f}"
        );
    }

    /// cmd_attach gibt attachment_id + URL nach erfolgreichem Upload aus
    #[test]
    fn test_cmd_attach_outputs_id_and_url() {
        let s = script();
        let f = extract_fn(&s, "cmd_attach");
        let has_id_output =
            f.contains(".id") || f.contains("attachment_id") || f.contains("\"id\"");
        let has_url_output = f.contains(".url") || f.contains("\"url\"");
        assert!(
            has_id_output && has_url_output,
            "cmd_attach muss attachment_id und URL nach Upload ausgeben, gefunden:\n{f}"
        );
    }

    // ─── cmd_attachments Tests ───────────────────────────────────────────────

    /// cmd_attachments existiert und ruft GET /attachments auf
    #[test]
    fn test_cmd_attachments_exists_and_calls_list_api() {
        let s = script();
        let f = extract_fn(&s, "cmd_attachments");
        assert!(
            !f.is_empty(),
            "cmd_attachments() muss im CLI-Script vorhanden sein"
        );
        assert!(
            f.contains("/attachments"),
            "cmd_attachments muss GET /api/projects/:id/tasks/:task_id/attachments aufrufen, gefunden:\n{f}"
        );
    }

    /// cmd_attachments validiert Argumente
    #[test]
    fn test_cmd_attachments_validates_args() {
        let s = script();
        let f = extract_fn(&s, "cmd_attachments");
        assert!(
            f.contains("Usage") || f.contains("usage"),
            "cmd_attachments muss eine Usage-Hilfe ausgeben wenn Argumente fehlen:\n{f}"
        );
        assert!(
            f.contains("exit 1"),
            "cmd_attachments muss mit exit 1 enden wenn Argumente fehlen:\n{f}"
        );
    }

    /// cmd_attachments gibt tabellarische Ausgabe aus (ID + Dateiname + Typ + Größe + Datum)
    #[test]
    fn test_cmd_attachments_tabular_output() {
        let s = script();
        let f = extract_fn(&s, "cmd_attachments");
        // Tabellenkopf oder jq-Formatierung mit mehreren Spalten
        let has_table = f.contains("filename")
            || f.contains("DATEINAME")
            || f.contains("NAME")
            || f.contains("\\t")
            || f.contains("printf");
        assert!(
            has_table,
            "cmd_attachments muss tabellarische Ausgabe mit Dateiname, Typ, Größe, Datum ausgeben, gefunden:\n{f}"
        );
    }

    // ─── cmd_download Tests ──────────────────────────────────────────────────

    /// cmd_download existiert und ruft GET /attachments/:id auf
    #[test]
    fn test_cmd_download_exists_and_calls_download_api() {
        let s = script();
        let f = extract_fn(&s, "cmd_download");
        assert!(
            !f.is_empty(),
            "cmd_download() muss im CLI-Script vorhanden sein"
        );
        assert!(
            f.contains("/attachments/"),
            "cmd_download muss GET /api/projects/:id/tasks/:task_id/attachments/:attachment_id aufrufen, gefunden:\n{f}"
        );
    }

    /// cmd_download validiert Argumente (project/task + attachment-id pflicht)
    #[test]
    fn test_cmd_download_validates_args() {
        let s = script();
        let f = extract_fn(&s, "cmd_download");
        assert!(
            f.contains("Usage") || f.contains("usage"),
            "cmd_download muss eine Usage-Hilfe ausgeben wenn Argumente fehlen:\n{f}"
        );
        assert!(
            f.contains("exit 1"),
            "cmd_download muss mit exit 1 enden wenn Argumente fehlen:\n{f}"
        );
    }

    /// cmd_download unterstützt optionalen Output-Pfad
    #[test]
    fn test_cmd_download_supports_optional_output_path() {
        let s = script();
        let f = extract_fn(&s, "cmd_download");
        // Optionaler Output-Pfad: entweder via -o Flag oder separate Variable
        let has_output = f.contains("output")
            || f.contains("outfile")
            || f.contains("-o ")
            || f.contains("out_path");
        assert!(
            has_output,
            "cmd_download muss optionalen Output-Pfad unterstützen (default: Dateiname aus Server), gefunden:\n{f}"
        );
    }

    // ─── Dispatch-Registrierung Tests ────────────────────────────────────────

    /// `attach` ist im main case-Dispatch registriert
    #[test]
    fn test_attach_registered_in_main() {
        let s = script();
        assert!(
            s.contains("attach)") || s.contains("\"attach\")"),
            "Der 'attach'-Subcommand muss im main case-Dispatch registriert sein"
        );
    }

    /// `attachments` ist im main case-Dispatch registriert
    #[test]
    fn test_attachments_registered_in_main() {
        let s = script();
        assert!(
            s.contains("attachments)") || s.contains("\"attachments\")"),
            "Der 'attachments'-Subcommand muss im main case-Dispatch registriert sein"
        );
    }

    /// `download` ist im main case-Dispatch registriert
    #[test]
    fn test_download_registered_in_main() {
        let s = script();
        assert!(
            s.contains("download)") || s.contains("\"download\")"),
            "Der 'download'-Subcommand muss im main case-Dispatch registriert sein"
        );
    }

    /// VERSION wird auf 0.3.0 erhöht
    #[test]
    fn test_version_is_0_3_0() {
        let s = script();
        assert!(
            s.contains("VERSION=\"0.3.0\""),
            "VERSION muss auf 0.3.0 gesetzt sein für den v0.3.0 Release, aktuell:\n{}",
            s.lines()
                .find(|l| l.contains("VERSION="))
                .unwrap_or("(keine VERSION-Zeile gefunden)")
        );
    }

    /// Help-Text dokumentiert attach, attachments und download
    #[test]
    fn test_help_contains_attachment_subcommands() {
        let s = script();
        let help_fn = extract_fn(&s, "cmd_help");
        assert!(
            help_fn.contains("attach") && help_fn.contains("download"),
            "cmd_help muss die neuen Attachment-Subcommands dokumentieren, gefunden:\n{help_fn}"
        );
    }
}
