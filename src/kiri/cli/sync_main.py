"""kiri-sync — manage git sync for kiri notes."""

import argparse

from kiri.sync import commit_notes, init_notes_repo, push_notes, status


def main():
    parser = argparse.ArgumentParser(description="Kiri notes git sync")
    parser.add_argument("--init", metavar="URL",
                        help="Initialize ~/kiri/ as a git repo with GitHub remote URL")
    parser.add_argument("--push", action="store_true",
                        help="Commit any changes and push to remote")
    parser.add_argument("--commit", action="store_true",
                        help="Commit changes locally (no push)")
    parser.add_argument("--status", action="store_true",
                        help="Show sync status")
    args = parser.parse_args()

    if args.init:
        init_notes_repo(args.init)
        return

    if args.push:
        commit_notes()
        push_notes()
        return

    if args.commit:
        if commit_notes():
            print("Committed.")
        else:
            print("Nothing to commit.")
        return

    if args.status:
        print(status())
        return

    # Default: show status
    print(status())


if __name__ == "__main__":
    main()
