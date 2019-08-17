#!/usr/bin/env python3.7

import argparse
import json
import os
import requests
import sys


CACHE = '~/.mr.cache'
GITLAB = os.getenv('GITLAB_URL', 'https://gitlab.com')


def eprint(msg):
    print(msg, file=sys.stderr)


def inc(session, args):
    groups = session.get(f'{GITLAB}/api/v4/groups').json()
    
    merge_requests = []
    project_names = {}

    for group in groups:
        params = {
            'state': 'opened'
        }

        grp_merge_requests = \
            session.get(f'{GITLAB}/api/v4/groups/{group["id"]}/merge_requests',
                        params=params).json()

        for mr in grp_merge_requests:
            merge_requests.append((group, mr))

    cache = []
    for (group, mr) in merge_requests:
        if mr["project_id"] not in project_names:
            project = \
                session.get(f'{GITLAB}/api/v4/projects/{mr["project_id"]}').json()
            project_names[mr["project_id"]] = project["name"]
        mr["project_name"] = project_names[mr["project_id"]]
        mr["group_name"] = group["name"]
        cache.append(mr)

    with open(os.path.expanduser(CACHE), 'w') as fp:
        json.dump(cache, fp, indent=2, sort_keys=True)


def show(session, args):
    with open(os.path.expanduser(CACHE), 'r') as fp:
        merge_requests = json.load(fp)

    if args.idx < 0:
        for (idx, mr) in enumerate(merge_requests):
            print(f'{(idx):>3}: [{mr["group_name"]}/{mr["project_name"]}] {mr["title"]}')
    else:
        try:
            mr = merge_requests[args.idx]

            username = mr["author"]["username"]
            url = mr["web_url"]

            print(f'[{mr["group_name"]}/{mr["project_name"]}] {mr["title"]} - @{username}')
            print(f'     {url}')
        except IndexError:
            eprint("Invalid merge request: {args.idx} is larger than {len(merge_reqests) - 1}")


def main():
    PRIVATE_TOKEN = os.getenv('GITLAB_PRIVATE_TOKEN')
    if not PRIVATE_TOKEN:
        eprint('GITLAB_PRIVATE_TOKEN was not set')
        sys.exit(1)

    session = requests.Session()
    session.headers.update({
        'PRIVATE-TOKEN': PRIVATE_TOKEN
    })
    session.hooks = {
        'response': lambda r, *args, **kwargs: r.raise_for_status()
    }

    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers()
    parser_inc = subparsers.add_parser('inc')
    parser_inc.set_defaults(func=inc)
    parser_show = subparsers.add_parser('show')
    parser_show.set_defaults(func=show)
    parser_show.add_argument('idx', type=int, nargs='?', default=-1)

    args = parser.parse_args()

    if 'func' in args:
        args.func(session, args)
    else:
        parser.print_usage()
        sys.exit(1)


if __name__ == '__main__':
    main()
