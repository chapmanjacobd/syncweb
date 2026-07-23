_syncweb() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="syncweb"
                ;;
            syncweb,attest)
                cmd="syncweb__subcmd__attest"
                ;;
            syncweb,automatic)
                cmd="syncweb__subcmd__automatic"
                ;;
            syncweb,collection)
                cmd="syncweb__subcmd__collection"
                ;;
            syncweb,completions)
                cmd="syncweb__subcmd__completions"
                ;;
            syncweb,config)
                cmd="syncweb__subcmd__config"
                ;;
            syncweb,create)
                cmd="syncweb__subcmd__create"
                ;;
            syncweb,daemon-sync)
                cmd="syncweb__subcmd__daemon__subcmd__sync"
                ;;
            syncweb,devices)
                cmd="syncweb__subcmd__devices"
                ;;
            syncweb,download)
                cmd="syncweb__subcmd__download"
                ;;
            syncweb,find)
                cmd="syncweb__subcmd__find"
                ;;
            syncweb,folders)
                cmd="syncweb__subcmd__folders"
                ;;
            syncweb,health)
                cmd="syncweb__subcmd__health"
                ;;
            syncweb,help)
                cmd="syncweb__subcmd__help"
                ;;
            syncweb,import)
                cmd="syncweb__subcmd__import"
                ;;
            syncweb,indexing)
                cmd="syncweb__subcmd__indexing"
                ;;
            syncweb,init)
                cmd="syncweb__subcmd__init"
                ;;
            syncweb,join)
                cmd="syncweb__subcmd__join"
                ;;
            syncweb,leave)
                cmd="syncweb__subcmd__leave"
                ;;
            syncweb,link)
                cmd="syncweb__subcmd__link"
                ;;
            syncweb,ls)
                cmd="syncweb__subcmd__ls"
                ;;
            syncweb,manpages)
                cmd="syncweb__subcmd__manpages"
                ;;
            syncweb,moderation)
                cmd="syncweb__subcmd__moderation"
                ;;
            syncweb,network)
                cmd="syncweb__subcmd__network"
                ;;
            syncweb,package)
                cmd="syncweb__subcmd__package"
                ;;
            syncweb,provider)
                cmd="syncweb__subcmd__provider"
                ;;
            syncweb,publish)
                cmd="syncweb__subcmd__publish"
                ;;
            syncweb,reload)
                cmd="syncweb__subcmd__reload"
                ;;
            syncweb,report)
                cmd="syncweb__subcmd__report"
                ;;
            syncweb,schedule)
                cmd="syncweb__subcmd__schedule"
                ;;
            syncweb,shutdown)
                cmd="syncweb__subcmd__shutdown"
                ;;
            syncweb,snapshot)
                cmd="syncweb__subcmd__snapshot"
                ;;
            syncweb,sort)
                cmd="syncweb__subcmd__sort"
                ;;
            syncweb,start)
                cmd="syncweb__subcmd__start"
                ;;
            syncweb,stat)
                cmd="syncweb__subcmd__stat"
                ;;
            syncweb,stats)
                cmd="syncweb__subcmd__stats"
                ;;
            syncweb,status)
                cmd="syncweb__subcmd__status"
                ;;
            syncweb,subscribe)
                cmd="syncweb__subcmd__subscribe"
                ;;
            syncweb,trust)
                cmd="syncweb__subcmd__trust"
                ;;
            syncweb,unpublish)
                cmd="syncweb__subcmd__unpublish"
                ;;
            syncweb,unsubscribe)
                cmd="syncweb__subcmd__unsubscribe"
                ;;
            syncweb,unwatch)
                cmd="syncweb__subcmd__unwatch"
                ;;
            syncweb,verify)
                cmd="syncweb__subcmd__verify"
                ;;
            syncweb,version)
                cmd="syncweb__subcmd__version"
                ;;
            syncweb,watch)
                cmd="syncweb__subcmd__watch"
                ;;
            syncweb__subcmd__collection,add)
                cmd="syncweb__subcmd__collection__subcmd__add"
                ;;
            syncweb__subcmd__collection,help)
                cmd="syncweb__subcmd__collection__subcmd__help"
                ;;
            syncweb__subcmd__collection,init)
                cmd="syncweb__subcmd__collection__subcmd__init"
                ;;
            syncweb__subcmd__collection,publish)
                cmd="syncweb__subcmd__collection__subcmd__publish"
                ;;
            syncweb__subcmd__collection,versions)
                cmd="syncweb__subcmd__collection__subcmd__versions"
                ;;
            syncweb__subcmd__collection__subcmd__help,add)
                cmd="syncweb__subcmd__collection__subcmd__help__subcmd__add"
                ;;
            syncweb__subcmd__collection__subcmd__help,help)
                cmd="syncweb__subcmd__collection__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__collection__subcmd__help,init)
                cmd="syncweb__subcmd__collection__subcmd__help__subcmd__init"
                ;;
            syncweb__subcmd__collection__subcmd__help,publish)
                cmd="syncweb__subcmd__collection__subcmd__help__subcmd__publish"
                ;;
            syncweb__subcmd__collection__subcmd__help,versions)
                cmd="syncweb__subcmd__collection__subcmd__help__subcmd__versions"
                ;;
            syncweb__subcmd__config,help)
                cmd="syncweb__subcmd__config__subcmd__help"
                ;;
            syncweb__subcmd__config,set)
                cmd="syncweb__subcmd__config__subcmd__set"
                ;;
            syncweb__subcmd__config,show)
                cmd="syncweb__subcmd__config__subcmd__show"
                ;;
            syncweb__subcmd__config__subcmd__help,help)
                cmd="syncweb__subcmd__config__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__config__subcmd__help,set)
                cmd="syncweb__subcmd__config__subcmd__help__subcmd__set"
                ;;
            syncweb__subcmd__config__subcmd__help,show)
                cmd="syncweb__subcmd__config__subcmd__help__subcmd__show"
                ;;
            syncweb__subcmd__help,attest)
                cmd="syncweb__subcmd__help__subcmd__attest"
                ;;
            syncweb__subcmd__help,automatic)
                cmd="syncweb__subcmd__help__subcmd__automatic"
                ;;
            syncweb__subcmd__help,collection)
                cmd="syncweb__subcmd__help__subcmd__collection"
                ;;
            syncweb__subcmd__help,completions)
                cmd="syncweb__subcmd__help__subcmd__completions"
                ;;
            syncweb__subcmd__help,config)
                cmd="syncweb__subcmd__help__subcmd__config"
                ;;
            syncweb__subcmd__help,create)
                cmd="syncweb__subcmd__help__subcmd__create"
                ;;
            syncweb__subcmd__help,daemon-sync)
                cmd="syncweb__subcmd__help__subcmd__daemon__subcmd__sync"
                ;;
            syncweb__subcmd__help,devices)
                cmd="syncweb__subcmd__help__subcmd__devices"
                ;;
            syncweb__subcmd__help,download)
                cmd="syncweb__subcmd__help__subcmd__download"
                ;;
            syncweb__subcmd__help,find)
                cmd="syncweb__subcmd__help__subcmd__find"
                ;;
            syncweb__subcmd__help,folders)
                cmd="syncweb__subcmd__help__subcmd__folders"
                ;;
            syncweb__subcmd__help,health)
                cmd="syncweb__subcmd__help__subcmd__health"
                ;;
            syncweb__subcmd__help,help)
                cmd="syncweb__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__help,import)
                cmd="syncweb__subcmd__help__subcmd__import"
                ;;
            syncweb__subcmd__help,indexing)
                cmd="syncweb__subcmd__help__subcmd__indexing"
                ;;
            syncweb__subcmd__help,init)
                cmd="syncweb__subcmd__help__subcmd__init"
                ;;
            syncweb__subcmd__help,join)
                cmd="syncweb__subcmd__help__subcmd__join"
                ;;
            syncweb__subcmd__help,leave)
                cmd="syncweb__subcmd__help__subcmd__leave"
                ;;
            syncweb__subcmd__help,link)
                cmd="syncweb__subcmd__help__subcmd__link"
                ;;
            syncweb__subcmd__help,ls)
                cmd="syncweb__subcmd__help__subcmd__ls"
                ;;
            syncweb__subcmd__help,manpages)
                cmd="syncweb__subcmd__help__subcmd__manpages"
                ;;
            syncweb__subcmd__help,moderation)
                cmd="syncweb__subcmd__help__subcmd__moderation"
                ;;
            syncweb__subcmd__help,network)
                cmd="syncweb__subcmd__help__subcmd__network"
                ;;
            syncweb__subcmd__help,package)
                cmd="syncweb__subcmd__help__subcmd__package"
                ;;
            syncweb__subcmd__help,provider)
                cmd="syncweb__subcmd__help__subcmd__provider"
                ;;
            syncweb__subcmd__help,publish)
                cmd="syncweb__subcmd__help__subcmd__publish"
                ;;
            syncweb__subcmd__help,reload)
                cmd="syncweb__subcmd__help__subcmd__reload"
                ;;
            syncweb__subcmd__help,report)
                cmd="syncweb__subcmd__help__subcmd__report"
                ;;
            syncweb__subcmd__help,schedule)
                cmd="syncweb__subcmd__help__subcmd__schedule"
                ;;
            syncweb__subcmd__help,shutdown)
                cmd="syncweb__subcmd__help__subcmd__shutdown"
                ;;
            syncweb__subcmd__help,snapshot)
                cmd="syncweb__subcmd__help__subcmd__snapshot"
                ;;
            syncweb__subcmd__help,sort)
                cmd="syncweb__subcmd__help__subcmd__sort"
                ;;
            syncweb__subcmd__help,start)
                cmd="syncweb__subcmd__help__subcmd__start"
                ;;
            syncweb__subcmd__help,stat)
                cmd="syncweb__subcmd__help__subcmd__stat"
                ;;
            syncweb__subcmd__help,stats)
                cmd="syncweb__subcmd__help__subcmd__stats"
                ;;
            syncweb__subcmd__help,status)
                cmd="syncweb__subcmd__help__subcmd__status"
                ;;
            syncweb__subcmd__help,subscribe)
                cmd="syncweb__subcmd__help__subcmd__subscribe"
                ;;
            syncweb__subcmd__help,trust)
                cmd="syncweb__subcmd__help__subcmd__trust"
                ;;
            syncweb__subcmd__help,unpublish)
                cmd="syncweb__subcmd__help__subcmd__unpublish"
                ;;
            syncweb__subcmd__help,unsubscribe)
                cmd="syncweb__subcmd__help__subcmd__unsubscribe"
                ;;
            syncweb__subcmd__help,unwatch)
                cmd="syncweb__subcmd__help__subcmd__unwatch"
                ;;
            syncweb__subcmd__help,verify)
                cmd="syncweb__subcmd__help__subcmd__verify"
                ;;
            syncweb__subcmd__help,version)
                cmd="syncweb__subcmd__help__subcmd__version"
                ;;
            syncweb__subcmd__help,watch)
                cmd="syncweb__subcmd__help__subcmd__watch"
                ;;
            syncweb__subcmd__help__subcmd__collection,add)
                cmd="syncweb__subcmd__help__subcmd__collection__subcmd__add"
                ;;
            syncweb__subcmd__help__subcmd__collection,init)
                cmd="syncweb__subcmd__help__subcmd__collection__subcmd__init"
                ;;
            syncweb__subcmd__help__subcmd__collection,publish)
                cmd="syncweb__subcmd__help__subcmd__collection__subcmd__publish"
                ;;
            syncweb__subcmd__help__subcmd__collection,versions)
                cmd="syncweb__subcmd__help__subcmd__collection__subcmd__versions"
                ;;
            syncweb__subcmd__help__subcmd__config,set)
                cmd="syncweb__subcmd__help__subcmd__config__subcmd__set"
                ;;
            syncweb__subcmd__help__subcmd__config,show)
                cmd="syncweb__subcmd__help__subcmd__config__subcmd__show"
                ;;
            syncweb__subcmd__help__subcmd__indexing,disable)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__disable"
                ;;
            syncweb__subcmd__help__subcmd__indexing,enable)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__enable"
                ;;
            syncweb__subcmd__help__subcmd__indexing,filter)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__filter"
                ;;
            syncweb__subcmd__help__subcmd__indexing,health)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__health"
                ;;
            syncweb__subcmd__help__subcmd__indexing,meta)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__meta"
                ;;
            syncweb__subcmd__help__subcmd__indexing,publish)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__publish"
                ;;
            syncweb__subcmd__help__subcmd__indexing,search)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__search"
                ;;
            syncweb__subcmd__help__subcmd__indexing__subcmd__filter,add)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__add"
                ;;
            syncweb__subcmd__help__subcmd__indexing__subcmd__filter,subscribe)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__subscribe"
                ;;
            syncweb__subcmd__help__subcmd__indexing__subcmd__meta,add)
                cmd="syncweb__subcmd__help__subcmd__indexing__subcmd__meta__subcmd__add"
                ;;
            syncweb__subcmd__help__subcmd__link,create)
                cmd="syncweb__subcmd__help__subcmd__link__subcmd__create"
                ;;
            syncweb__subcmd__help__subcmd__link,resolve)
                cmd="syncweb__subcmd__help__subcmd__link__subcmd__resolve"
                ;;
            syncweb__subcmd__help__subcmd__link,revoke)
                cmd="syncweb__subcmd__help__subcmd__link__subcmd__revoke"
                ;;
            syncweb__subcmd__help__subcmd__moderation,hide)
                cmd="syncweb__subcmd__help__subcmd__moderation__subcmd__hide"
                ;;
            syncweb__subcmd__help__subcmd__moderation,ls)
                cmd="syncweb__subcmd__help__subcmd__moderation__subcmd__ls"
                ;;
            syncweb__subcmd__help__subcmd__network,create)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__create"
                ;;
            syncweb__subcmd__help__subcmd__network,invite)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__invite"
                ;;
            syncweb__subcmd__help__subcmd__network,join)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__join"
                ;;
            syncweb__subcmd__help__subcmd__network,kick)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__kick"
                ;;
            syncweb__subcmd__help__subcmd__network,leave)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__leave"
                ;;
            syncweb__subcmd__help__subcmd__network,ls)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__ls"
                ;;
            syncweb__subcmd__help__subcmd__network,test-relay)
                cmd="syncweb__subcmd__help__subcmd__network__subcmd__test__subcmd__relay"
                ;;
            syncweb__subcmd__help__subcmd__package,export)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__export"
                ;;
            syncweb__subcmd__help__subcmd__package,import)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__import"
                ;;
            syncweb__subcmd__help__subcmd__package,info)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__info"
                ;;
            syncweb__subcmd__help__subcmd__package,install)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__install"
                ;;
            syncweb__subcmd__help__subcmd__package,list)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__list"
                ;;
            syncweb__subcmd__help__subcmd__package,remove)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__remove"
                ;;
            syncweb__subcmd__help__subcmd__package,search)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__search"
                ;;
            syncweb__subcmd__help__subcmd__package,switch)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__switch"
                ;;
            syncweb__subcmd__help__subcmd__package,upgrade)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__upgrade"
                ;;
            syncweb__subcmd__help__subcmd__package,verify)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__verify"
                ;;
            syncweb__subcmd__help__subcmd__package,versions)
                cmd="syncweb__subcmd__help__subcmd__package__subcmd__versions"
                ;;
            syncweb__subcmd__help__subcmd__provider,add)
                cmd="syncweb__subcmd__help__subcmd__provider__subcmd__add"
                ;;
            syncweb__subcmd__help__subcmd__schedule,folder)
                cmd="syncweb__subcmd__help__subcmd__schedule__subcmd__folder"
                ;;
            syncweb__subcmd__help__subcmd__schedule,set)
                cmd="syncweb__subcmd__help__subcmd__schedule__subcmd__set"
                ;;
            syncweb__subcmd__help__subcmd__snapshot,create)
                cmd="syncweb__subcmd__help__subcmd__snapshot__subcmd__create"
                ;;
            syncweb__subcmd__help__subcmd__snapshot,delete)
                cmd="syncweb__subcmd__help__subcmd__snapshot__subcmd__delete"
                ;;
            syncweb__subcmd__help__subcmd__snapshot,diff)
                cmd="syncweb__subcmd__help__subcmd__snapshot__subcmd__diff"
                ;;
            syncweb__subcmd__help__subcmd__snapshot,list)
                cmd="syncweb__subcmd__help__subcmd__snapshot__subcmd__list"
                ;;
            syncweb__subcmd__help__subcmd__snapshot,restore)
                cmd="syncweb__subcmd__help__subcmd__snapshot__subcmd__restore"
                ;;
            syncweb__subcmd__help__subcmd__trust,delegate)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__delegate"
                ;;
            syncweb__subcmd__help__subcmd__trust,provider)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider"
                ;;
            syncweb__subcmd__help__subcmd__trust,show)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__show"
                ;;
            syncweb__subcmd__help__subcmd__trust,stream)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__stream"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__provider,ban)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__ban"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__provider,distrust)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__distrust"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__provider,list)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__list"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__provider,show)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__show"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__provider,unban)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__unban"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__provider,vouch)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__vouch"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__stream,publish)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__stream__subcmd__publish"
                ;;
            syncweb__subcmd__help__subcmd__trust__subcmd__stream,subscribe)
                cmd="syncweb__subcmd__help__subcmd__trust__subcmd__stream__subcmd__subscribe"
                ;;
            syncweb__subcmd__indexing,disable)
                cmd="syncweb__subcmd__indexing__subcmd__disable"
                ;;
            syncweb__subcmd__indexing,enable)
                cmd="syncweb__subcmd__indexing__subcmd__enable"
                ;;
            syncweb__subcmd__indexing,filter)
                cmd="syncweb__subcmd__indexing__subcmd__filter"
                ;;
            syncweb__subcmd__indexing,health)
                cmd="syncweb__subcmd__indexing__subcmd__health"
                ;;
            syncweb__subcmd__indexing,help)
                cmd="syncweb__subcmd__indexing__subcmd__help"
                ;;
            syncweb__subcmd__indexing,meta)
                cmd="syncweb__subcmd__indexing__subcmd__meta"
                ;;
            syncweb__subcmd__indexing,publish)
                cmd="syncweb__subcmd__indexing__subcmd__publish"
                ;;
            syncweb__subcmd__indexing,search)
                cmd="syncweb__subcmd__indexing__subcmd__search"
                ;;
            syncweb__subcmd__indexing__subcmd__filter,add)
                cmd="syncweb__subcmd__indexing__subcmd__filter__subcmd__add"
                ;;
            syncweb__subcmd__indexing__subcmd__filter,help)
                cmd="syncweb__subcmd__indexing__subcmd__filter__subcmd__help"
                ;;
            syncweb__subcmd__indexing__subcmd__filter,subscribe)
                cmd="syncweb__subcmd__indexing__subcmd__filter__subcmd__subscribe"
                ;;
            syncweb__subcmd__indexing__subcmd__filter__subcmd__help,add)
                cmd="syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__add"
                ;;
            syncweb__subcmd__indexing__subcmd__filter__subcmd__help,help)
                cmd="syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__indexing__subcmd__filter__subcmd__help,subscribe)
                cmd="syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__subscribe"
                ;;
            syncweb__subcmd__indexing__subcmd__help,disable)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__disable"
                ;;
            syncweb__subcmd__indexing__subcmd__help,enable)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__enable"
                ;;
            syncweb__subcmd__indexing__subcmd__help,filter)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__filter"
                ;;
            syncweb__subcmd__indexing__subcmd__help,health)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__health"
                ;;
            syncweb__subcmd__indexing__subcmd__help,help)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__indexing__subcmd__help,meta)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__meta"
                ;;
            syncweb__subcmd__indexing__subcmd__help,publish)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__publish"
                ;;
            syncweb__subcmd__indexing__subcmd__help,search)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__search"
                ;;
            syncweb__subcmd__indexing__subcmd__help__subcmd__filter,add)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__add"
                ;;
            syncweb__subcmd__indexing__subcmd__help__subcmd__filter,subscribe)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__subscribe"
                ;;
            syncweb__subcmd__indexing__subcmd__help__subcmd__meta,add)
                cmd="syncweb__subcmd__indexing__subcmd__help__subcmd__meta__subcmd__add"
                ;;
            syncweb__subcmd__indexing__subcmd__meta,add)
                cmd="syncweb__subcmd__indexing__subcmd__meta__subcmd__add"
                ;;
            syncweb__subcmd__indexing__subcmd__meta,help)
                cmd="syncweb__subcmd__indexing__subcmd__meta__subcmd__help"
                ;;
            syncweb__subcmd__indexing__subcmd__meta__subcmd__help,add)
                cmd="syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__add"
                ;;
            syncweb__subcmd__indexing__subcmd__meta__subcmd__help,help)
                cmd="syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__link,create)
                cmd="syncweb__subcmd__link__subcmd__create"
                ;;
            syncweb__subcmd__link,help)
                cmd="syncweb__subcmd__link__subcmd__help"
                ;;
            syncweb__subcmd__link,resolve)
                cmd="syncweb__subcmd__link__subcmd__resolve"
                ;;
            syncweb__subcmd__link,revoke)
                cmd="syncweb__subcmd__link__subcmd__revoke"
                ;;
            syncweb__subcmd__link__subcmd__help,create)
                cmd="syncweb__subcmd__link__subcmd__help__subcmd__create"
                ;;
            syncweb__subcmd__link__subcmd__help,help)
                cmd="syncweb__subcmd__link__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__link__subcmd__help,resolve)
                cmd="syncweb__subcmd__link__subcmd__help__subcmd__resolve"
                ;;
            syncweb__subcmd__link__subcmd__help,revoke)
                cmd="syncweb__subcmd__link__subcmd__help__subcmd__revoke"
                ;;
            syncweb__subcmd__moderation,help)
                cmd="syncweb__subcmd__moderation__subcmd__help"
                ;;
            syncweb__subcmd__moderation,hide)
                cmd="syncweb__subcmd__moderation__subcmd__hide"
                ;;
            syncweb__subcmd__moderation,ls)
                cmd="syncweb__subcmd__moderation__subcmd__ls"
                ;;
            syncweb__subcmd__moderation__subcmd__help,help)
                cmd="syncweb__subcmd__moderation__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__moderation__subcmd__help,hide)
                cmd="syncweb__subcmd__moderation__subcmd__help__subcmd__hide"
                ;;
            syncweb__subcmd__moderation__subcmd__help,ls)
                cmd="syncweb__subcmd__moderation__subcmd__help__subcmd__ls"
                ;;
            syncweb__subcmd__network,create)
                cmd="syncweb__subcmd__network__subcmd__create"
                ;;
            syncweb__subcmd__network,help)
                cmd="syncweb__subcmd__network__subcmd__help"
                ;;
            syncweb__subcmd__network,invite)
                cmd="syncweb__subcmd__network__subcmd__invite"
                ;;
            syncweb__subcmd__network,join)
                cmd="syncweb__subcmd__network__subcmd__join"
                ;;
            syncweb__subcmd__network,kick)
                cmd="syncweb__subcmd__network__subcmd__kick"
                ;;
            syncweb__subcmd__network,leave)
                cmd="syncweb__subcmd__network__subcmd__leave"
                ;;
            syncweb__subcmd__network,ls)
                cmd="syncweb__subcmd__network__subcmd__ls"
                ;;
            syncweb__subcmd__network,test-relay)
                cmd="syncweb__subcmd__network__subcmd__test__subcmd__relay"
                ;;
            syncweb__subcmd__network__subcmd__help,create)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__create"
                ;;
            syncweb__subcmd__network__subcmd__help,help)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__network__subcmd__help,invite)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__invite"
                ;;
            syncweb__subcmd__network__subcmd__help,join)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__join"
                ;;
            syncweb__subcmd__network__subcmd__help,kick)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__kick"
                ;;
            syncweb__subcmd__network__subcmd__help,leave)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__leave"
                ;;
            syncweb__subcmd__network__subcmd__help,ls)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__ls"
                ;;
            syncweb__subcmd__network__subcmd__help,test-relay)
                cmd="syncweb__subcmd__network__subcmd__help__subcmd__test__subcmd__relay"
                ;;
            syncweb__subcmd__package,export)
                cmd="syncweb__subcmd__package__subcmd__export"
                ;;
            syncweb__subcmd__package,help)
                cmd="syncweb__subcmd__package__subcmd__help"
                ;;
            syncweb__subcmd__package,import)
                cmd="syncweb__subcmd__package__subcmd__import"
                ;;
            syncweb__subcmd__package,info)
                cmd="syncweb__subcmd__package__subcmd__info"
                ;;
            syncweb__subcmd__package,install)
                cmd="syncweb__subcmd__package__subcmd__install"
                ;;
            syncweb__subcmd__package,list)
                cmd="syncweb__subcmd__package__subcmd__list"
                ;;
            syncweb__subcmd__package,remove)
                cmd="syncweb__subcmd__package__subcmd__remove"
                ;;
            syncweb__subcmd__package,search)
                cmd="syncweb__subcmd__package__subcmd__search"
                ;;
            syncweb__subcmd__package,switch)
                cmd="syncweb__subcmd__package__subcmd__switch"
                ;;
            syncweb__subcmd__package,upgrade)
                cmd="syncweb__subcmd__package__subcmd__upgrade"
                ;;
            syncweb__subcmd__package,verify)
                cmd="syncweb__subcmd__package__subcmd__verify"
                ;;
            syncweb__subcmd__package,versions)
                cmd="syncweb__subcmd__package__subcmd__versions"
                ;;
            syncweb__subcmd__package__subcmd__help,export)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__export"
                ;;
            syncweb__subcmd__package__subcmd__help,help)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__package__subcmd__help,import)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__import"
                ;;
            syncweb__subcmd__package__subcmd__help,info)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__info"
                ;;
            syncweb__subcmd__package__subcmd__help,install)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__install"
                ;;
            syncweb__subcmd__package__subcmd__help,list)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__list"
                ;;
            syncweb__subcmd__package__subcmd__help,remove)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__remove"
                ;;
            syncweb__subcmd__package__subcmd__help,search)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__search"
                ;;
            syncweb__subcmd__package__subcmd__help,switch)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__switch"
                ;;
            syncweb__subcmd__package__subcmd__help,upgrade)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__upgrade"
                ;;
            syncweb__subcmd__package__subcmd__help,verify)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__verify"
                ;;
            syncweb__subcmd__package__subcmd__help,versions)
                cmd="syncweb__subcmd__package__subcmd__help__subcmd__versions"
                ;;
            syncweb__subcmd__provider,add)
                cmd="syncweb__subcmd__provider__subcmd__add"
                ;;
            syncweb__subcmd__provider,help)
                cmd="syncweb__subcmd__provider__subcmd__help"
                ;;
            syncweb__subcmd__provider__subcmd__help,add)
                cmd="syncweb__subcmd__provider__subcmd__help__subcmd__add"
                ;;
            syncweb__subcmd__provider__subcmd__help,help)
                cmd="syncweb__subcmd__provider__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__schedule,folder)
                cmd="syncweb__subcmd__schedule__subcmd__folder"
                ;;
            syncweb__subcmd__schedule,help)
                cmd="syncweb__subcmd__schedule__subcmd__help"
                ;;
            syncweb__subcmd__schedule,set)
                cmd="syncweb__subcmd__schedule__subcmd__set"
                ;;
            syncweb__subcmd__schedule__subcmd__help,folder)
                cmd="syncweb__subcmd__schedule__subcmd__help__subcmd__folder"
                ;;
            syncweb__subcmd__schedule__subcmd__help,help)
                cmd="syncweb__subcmd__schedule__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__schedule__subcmd__help,set)
                cmd="syncweb__subcmd__schedule__subcmd__help__subcmd__set"
                ;;
            syncweb__subcmd__snapshot,create)
                cmd="syncweb__subcmd__snapshot__subcmd__create"
                ;;
            syncweb__subcmd__snapshot,delete)
                cmd="syncweb__subcmd__snapshot__subcmd__delete"
                ;;
            syncweb__subcmd__snapshot,diff)
                cmd="syncweb__subcmd__snapshot__subcmd__diff"
                ;;
            syncweb__subcmd__snapshot,help)
                cmd="syncweb__subcmd__snapshot__subcmd__help"
                ;;
            syncweb__subcmd__snapshot,list)
                cmd="syncweb__subcmd__snapshot__subcmd__list"
                ;;
            syncweb__subcmd__snapshot,restore)
                cmd="syncweb__subcmd__snapshot__subcmd__restore"
                ;;
            syncweb__subcmd__snapshot__subcmd__help,create)
                cmd="syncweb__subcmd__snapshot__subcmd__help__subcmd__create"
                ;;
            syncweb__subcmd__snapshot__subcmd__help,delete)
                cmd="syncweb__subcmd__snapshot__subcmd__help__subcmd__delete"
                ;;
            syncweb__subcmd__snapshot__subcmd__help,diff)
                cmd="syncweb__subcmd__snapshot__subcmd__help__subcmd__diff"
                ;;
            syncweb__subcmd__snapshot__subcmd__help,help)
                cmd="syncweb__subcmd__snapshot__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__snapshot__subcmd__help,list)
                cmd="syncweb__subcmd__snapshot__subcmd__help__subcmd__list"
                ;;
            syncweb__subcmd__snapshot__subcmd__help,restore)
                cmd="syncweb__subcmd__snapshot__subcmd__help__subcmd__restore"
                ;;
            syncweb__subcmd__trust,delegate)
                cmd="syncweb__subcmd__trust__subcmd__delegate"
                ;;
            syncweb__subcmd__trust,help)
                cmd="syncweb__subcmd__trust__subcmd__help"
                ;;
            syncweb__subcmd__trust,provider)
                cmd="syncweb__subcmd__trust__subcmd__provider"
                ;;
            syncweb__subcmd__trust,show)
                cmd="syncweb__subcmd__trust__subcmd__show"
                ;;
            syncweb__subcmd__trust,stream)
                cmd="syncweb__subcmd__trust__subcmd__stream"
                ;;
            syncweb__subcmd__trust__subcmd__help,delegate)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__delegate"
                ;;
            syncweb__subcmd__trust__subcmd__help,help)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__trust__subcmd__help,provider)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider"
                ;;
            syncweb__subcmd__trust__subcmd__help,show)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__show"
                ;;
            syncweb__subcmd__trust__subcmd__help,stream)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__stream"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__provider,ban)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__ban"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__provider,distrust)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__distrust"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__provider,list)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__list"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__provider,show)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__show"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__provider,unban)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__unban"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__provider,vouch)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__vouch"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__stream,publish)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__stream__subcmd__publish"
                ;;
            syncweb__subcmd__trust__subcmd__help__subcmd__stream,subscribe)
                cmd="syncweb__subcmd__trust__subcmd__help__subcmd__stream__subcmd__subscribe"
                ;;
            syncweb__subcmd__trust__subcmd__provider,ban)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__ban"
                ;;
            syncweb__subcmd__trust__subcmd__provider,distrust)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__distrust"
                ;;
            syncweb__subcmd__trust__subcmd__provider,help)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help"
                ;;
            syncweb__subcmd__trust__subcmd__provider,list)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__list"
                ;;
            syncweb__subcmd__trust__subcmd__provider,show)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__show"
                ;;
            syncweb__subcmd__trust__subcmd__provider,unban)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__unban"
                ;;
            syncweb__subcmd__trust__subcmd__provider,vouch)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__vouch"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,ban)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__ban"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,distrust)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__distrust"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,help)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,list)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__list"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,show)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__show"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,unban)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__unban"
                ;;
            syncweb__subcmd__trust__subcmd__provider__subcmd__help,vouch)
                cmd="syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__vouch"
                ;;
            syncweb__subcmd__trust__subcmd__stream,help)
                cmd="syncweb__subcmd__trust__subcmd__stream__subcmd__help"
                ;;
            syncweb__subcmd__trust__subcmd__stream,publish)
                cmd="syncweb__subcmd__trust__subcmd__stream__subcmd__publish"
                ;;
            syncweb__subcmd__trust__subcmd__stream,subscribe)
                cmd="syncweb__subcmd__trust__subcmd__stream__subcmd__subscribe"
                ;;
            syncweb__subcmd__trust__subcmd__stream__subcmd__help,help)
                cmd="syncweb__subcmd__trust__subcmd__stream__subcmd__help__subcmd__help"
                ;;
            syncweb__subcmd__trust__subcmd__stream__subcmd__help,publish)
                cmd="syncweb__subcmd__trust__subcmd__stream__subcmd__help__subcmd__publish"
                ;;
            syncweb__subcmd__trust__subcmd__stream__subcmd__help,subscribe)
                cmd="syncweb__subcmd__trust__subcmd__stream__subcmd__help__subcmd__subscribe"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        syncweb)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help version start shutdown status reload daemon-sync unwatch create join leave unsubscribe folders devices config ls find sort stat download import snapshot health init automatic watch stats verify schedule subscribe publish unpublish collection package network indexing link provider trust attest report moderation completions manpages help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__attest)
            opts="-h --license --provenance --derivative --sequence --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --license)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --provenance)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --derivative)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sequence)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__automatic)
            opts="-h --show-filters --dry-run --paths --filters --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --paths)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filters)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help init add versions publish help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__add)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__help)
            opts="init add versions publish help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__help__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__help__subcmd__init)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__help__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__help__subcmd__versions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__init)
            opts="-h --version --name --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__publish)
            opts="-h --namespace --sequence --bootstrap --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --namespace)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sequence)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --bootstrap)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__collection__subcmd__versions)
            opts="-h --version --changelog --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --changelog)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__completions)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help bash elvish fish powershell zsh"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help set show help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config__subcmd__help)
            opts="set show help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config__subcmd__help__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config__subcmd__help__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config__subcmd__set)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__config__subcmd__show)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__create)
            opts="-h --mode --relay-fallback --network --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --mode)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__daemon__subcmd__sync)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__devices)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__download)
            opts="-h --max-peers --min-peers --min-count --max-count --threads --hash --provider --from --no-seeding --no-sharing --min-providers --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --max-peers)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-peers)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --provider)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-providers)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__find)
            opts="-i -s -F -p -H -L -a -d -e -h --kind --ignore-case --case-sensitive --fixed-strings --full-path --hidden --follow-links --absolute-path --download --depth --min-depth --max-depth --sizes --modified-within --modified-before --time-modified --extension --type --threads --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --kind)
                    COMPREPLY=($(compgen -W "exact glob regex" -- "${cur}"))
                    return 0
                    ;;
                --depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sizes)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --modified-within)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --modified-before)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --time-modified)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --extension)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -e)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --type)
                    COMPREPLY=($(compgen -W "f d l" -- "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__folders)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__health)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help)
            opts="version start shutdown status reload daemon-sync unwatch create join leave unsubscribe folders devices config ls find sort stat download import snapshot health init automatic watch stats verify schedule subscribe publish unpublish collection package network indexing link provider trust attest report moderation completions manpages help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__attest)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__automatic)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__collection)
            opts="init add versions publish"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__collection__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__collection__subcmd__init)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__collection__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__collection__subcmd__versions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__completions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__config)
            opts="set show"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__config__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__config__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__daemon__subcmd__sync)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__devices)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__find)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__folders)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__health)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing)
            opts="enable disable publish search health meta filter"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__disable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__enable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__filter)
            opts="add subscribe"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__filter__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__health)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__meta)
            opts="add"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__meta__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__indexing__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__init)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__join)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__leave)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__link)
            opts="create resolve revoke"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__link__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__link__subcmd__resolve)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__link__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__ls)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__manpages)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__moderation)
            opts="ls hide"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__moderation__subcmd__hide)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__moderation__subcmd__ls)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network)
            opts="create ls join leave invite kick test-relay"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__invite)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__join)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__kick)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__leave)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__ls)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__network__subcmd__test__subcmd__relay)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package)
            opts="export import search info install upgrade remove verify list versions switch"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__install)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__remove)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__switch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__upgrade)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__package__subcmd__versions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__provider)
            opts="add"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__provider__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__reload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__report)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__schedule)
            opts="set folder"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__schedule__subcmd__folder)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__schedule__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__shutdown)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__snapshot)
            opts="create restore list diff delete"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__snapshot__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__snapshot__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__snapshot__subcmd__diff)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__snapshot__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__snapshot__subcmd__restore)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__sort)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__start)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__stat)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust)
            opts="show delegate provider stream"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__delegate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider)
            opts="show list ban unban vouch distrust"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__ban)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__distrust)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__unban)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__provider__subcmd__vouch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__stream)
            opts="subscribe publish"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__stream__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__trust__subcmd__stream__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__unpublish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__unsubscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__unwatch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__version)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__help__subcmd__watch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__import)
            opts="-h --folder --threads --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --folder)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help enable disable publish search health meta filter help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__disable)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__enable)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help add subscribe help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter__subcmd__add)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help device file hash"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter__subcmd__help)
            opts="add subscribe help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter__subcmd__help__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__filter__subcmd__subscribe)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__health)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help)
            opts="enable disable publish search health meta filter help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__disable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__enable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__filter)
            opts="add subscribe"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__filter__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__health)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__meta)
            opts="add"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__meta__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__help__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__meta)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help add help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__meta__subcmd__add)
            opts="-h --sequence --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sequence)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__meta__subcmd__help)
            opts="add help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__meta__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__publish)
            opts="-h --catalog --tag --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --catalog)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tag)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__indexing__subcmd__search)
            opts="-h --limit --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__init)
            opts="-h --mode --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --mode)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__join)
            opts="-h --mode --relay-fallback --network --once --ingest-only --ignore-self --prefix --sync-prefix --glob --max-count --max-size --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --mode)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sync-prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --glob)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__leave)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help create resolve revoke help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__create)
            opts="-h --name --version --sequence --private --expires --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sequence)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expires)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__help)
            opts="create resolve revoke help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__help__subcmd__resolve)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__help__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__resolve)
            opts="-h --version --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__link__subcmd__revoke)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__ls)
            opts="-h --sort --threads --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sort)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__manpages)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help ls hide help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation__subcmd__help)
            opts="ls hide help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation__subcmd__help__subcmd__hide)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation__subcmd__help__subcmd__ls)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation__subcmd__hide)
            opts="-h --reason --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__moderation__subcmd__ls)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help create ls join leave invite kick test-relay help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__create)
            opts="-h --label --invite-only --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help)
            opts="create ls join leave invite kick test-relay help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__invite)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__join)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__kick)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__leave)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__ls)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__help__subcmd__test__subcmd__relay)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__invite)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__join)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__kick)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__leave)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__ls)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__network__subcmd__test__subcmd__relay)
            opts="-h --relay-url --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --relay-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help export import search info install upgrade remove verify list versions switch help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__export)
            opts="-h --version --filter --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help)
            opts="export import search info install upgrade remove verify list versions switch help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__install)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__remove)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__switch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__upgrade)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__help__subcmd__versions)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__import)
            opts="-h --filter --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__info)
            opts="-h --ticket --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ticket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__install)
            opts="-h --ticket --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ticket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__list)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__remove)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__search)
            opts="-h --bootstrap --timeout-ms --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --bootstrap)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout-ms)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__switch)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__upgrade)
            opts="-h --ticket --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ticket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__verify)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__package__subcmd__versions)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__provider)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help add help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__provider__subcmd__add)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__provider__subcmd__help)
            opts="add help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__provider__subcmd__help__subcmd__add)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__provider__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__publish)
            opts="-h --blob --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --blob)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__reload)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__report)
            opts="-h --reason --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help set folder help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule__subcmd__folder)
            opts="-h --active --max-upload --max-download --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --active)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-upload)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-download)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule__subcmd__help)
            opts="set folder help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule__subcmd__help__subcmd__folder)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule__subcmd__help__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__schedule__subcmd__set)
            opts="-h --active --bandwidth --period --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --active)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --bandwidth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --period)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__shutdown)
            opts="-h --force --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help create restore list diff delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__create)
            opts="-h --description --threads --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__delete)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__diff)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help)
            opts="create restore list diff delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help__subcmd__diff)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__help__subcmd__restore)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__list)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__snapshot__subcmd__restore)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__sort)
            opts="-h --by --min-seeders --max-seeders --niche --frecency-weight --limit-size --depth --min-depth --max-depth --threads --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --by)
                    COMPREPLY=($(compgen -W "niche frecency peers random folder time date week month year size folder-size folder-avg-size folder-date folder-time count" -- "${cur}"))
                    return 0
                    ;;
                --min-seeders)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-seeders)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --niche)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --frecency-weight)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__start)
            opts="-h --bg --data-dir --log-file --max-threads --sync-interval --verbose --json --embedded --no-daemon --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --log-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sync-interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__stat)
            opts="-h --terse --format --threads --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__stats)
            opts="-h --folder --peer --reset --period --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --folder)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --peer)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --period)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__status)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__subscribe)
            opts="-h --ingest-only --ignore-self --sync-prefix --glob --max-count --max-size --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --sync-prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --glob)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-count)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help show delegate provider stream help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__delegate)
            opts="-h --expires --scope --sequence --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --expires)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --scope)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sequence)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help)
            opts="show delegate provider stream help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__delegate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider)
            opts="show list ban unban vouch distrust"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__ban)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__distrust)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__unban)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__provider__subcmd__vouch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__stream)
            opts="subscribe publish"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__stream__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__help__subcmd__stream__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help show list ban unban vouch distrust help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__ban)
            opts="-h --hash --reason --duration --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --duration)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__distrust)
            opts="-h --scope --reason --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --scope)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help)
            opts="show list ban unban vouch distrust help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__ban)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__distrust)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__unban)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__help__subcmd__vouch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__list)
            opts="-h --hash --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__show)
            opts="-h --hash --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__unban)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__provider__subcmd__vouch)
            opts="-h --scope --reason --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --scope)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__show)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help subscribe publish help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream__subcmd__help)
            opts="subscribe publish help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream__subcmd__help__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream__subcmd__help__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream__subcmd__publish)
            opts="-h --provider --signal --hash --sequence --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --provider)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signal)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sequence)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__trust__subcmd__stream__subcmd__subscribe)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__unpublish)
            opts="-h --blob --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --blob)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__unsubscribe)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__unwatch)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__verify)
            opts="-h --hash --path-filter --glob-filter --fix --provider --from --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --path-filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --glob-filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --provider)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__version)
            opts="-h --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        syncweb__subcmd__watch)
            opts="-h --debounce-ms --exclude --once --verbose --json --embedded --no-daemon --data-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --debounce-ms)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --exclude)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _syncweb -o nosort -o bashdefault -o default syncweb
else
    complete -F _syncweb -o bashdefault -o default syncweb
fi
