# No, you can't have that role!

A simple discord bot to automatically kick people who get a certain role.

This can be used fairly simply. The role's name simply must contain `kick me`, and any time a user
gains that role they will be kicked.

> :warning: NOTE: Any role containing `kick me` will cause users to automatically
> be kicked, **even if they had the role already!**

This can provide an effective way to combat some types of selfbots, which simply
choose the first option in onboarding, so you can have choosing the first option
assign a role with `kick me` in the name and boom, kick them.

## Configuration

```shell
curl -O https://raw.githubusercontent.com/randomairborne/autokick/main/compose.yaml
nano compose.yaml # fill in your Discord bot's token, and uncomment the DISCORD_TOKEN field
docker compose up -d
```

## Example Discord Onboarding Setup

![Don't Pick It in onboarding!](.github/img/DontPick.png)
![Onboarding Dash!](.github/img/Onboarding.png)
![Not human!](.github/img/NotHuman.png)
