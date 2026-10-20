//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton, lain3d						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "Audio.h"

#include "../Configuration.h"
#include "../Gameplay/FieldEffectRules.h"

#ifdef WIN32
#define WIN32_LEAN_AND_MEAN
#include <bass.h>
#else
#include <AL/alc.h>
#endif

#include <nlnx/nx.hpp>
#include <nlnx/audio.hpp>

#include <algorithm>

namespace ms
{

	alure::DeviceManager Music::devMgr;
	alure::Device Music::dev;
	alure::Context Music::ctx;
	alure::Source Music::music_src;
	alure::Buffer Music::music_buff;
	float Music::volume = 1.0f;
	std::string Music::pending;
	std::chrono::steady_clock::time_point Music::fade_start;
	int32_t Music::fade_length = 0;
	int32_t Music::start_delay = 0;
	int32_t Music::fadein_length = 0;
	bool Music::looping = true;
	bool Music::fading_in = false;
	std::chrono::steady_clock::time_point Music::fadein_start;
	std::unordered_map<std::string, membuf *> Music::audiodb;
	std::vector<alure::Source> Sound::sources;
	float Sound::gain = 1.0f;

	alure::Source Sound::free_source()
	{
		// Sources are shared between sounds and only taken while one plays,
		// so two sounds started together, like a mob's last hit and its
		// death cry, never cut each other off.
		for (auto &source : sources)
			if (!source.isPlaying())
				return source;

		// OpenAL allows 256 sources per context; past the cap the sound is
		// dropped.
		if (sources.size() >= MAX_SOURCES)
			return alure::Source();

		sources.push_back(Music::ctx.createSource());

		return sources.back();
	}

	Sound::Sound(Name name)
	{
		id = soundids[name];
	}

	Sound::Sound(int32_t itemid)
	{
		auto fitemid = format_id(itemid);

		if (itemids.find(fitemid) != itemids.end())
		{
			id = itemids.at(fitemid);
		} else
		{
			auto pid = (10000 * (itemid / 10000));
			auto fpid = format_id(pid);

			if (itemids.find(fpid) != itemids.end())
				id = itemids.at(fpid);
			else
				id = itemids.at("02000000");
		}
	}

	Sound::Sound(nl::node src)
	{
		id = add_sound(src);
	}

	Sound::Sound()
	{
		id = 0;
	}

	void Sound::play()
	{
		play(100);
	}

	void Sound::play(uint8_t volume)
	{
		if (id > 0)
		{
			std::string id_s = std::to_string((uint32_t) id);
			alure::Buffer buff = Music::ctx.getBuffer(id_s);

			if (alure::Source source = free_source())
			{
				source.setGain(gain * std::min<uint8_t>(volume, 100) / 100.0f);
				source.setLooping(false);
				source.play(buff);
			}
		}
	}

	void Sound::play_loop()
	{
		stop();

		if (id > 0)
		{
			std::string id_s = std::to_string((uint32_t) id);
			alure::Buffer buff = Music::ctx.getBuffer(id_s);

			if (alure::Source source = free_source())
			{
				source.setGain(gain);
				source.setLooping(true);
				source.play(buff);
				looping = source;
			}
		}
	}

	void Sound::stop()
	{
		if (looping)
		{
			looping.stop();
			looping = alure::Source();
		}
	}

	Error Sound::init()
	{
		//if (!BASS_Init(1, 44100, 0, nullptr, 0))
		//	return Error::Code::AUDIO;

		nl::node uisrc = nl::nx::sound["UI.img"];

		add_sound(Sound::Name::BUTTONCLICK, uisrc["BtMouseClick"]);
		add_sound(Sound::Name::BUTTONOVER, uisrc["BtMouseOver"]);
		add_sound(Sound::Name::CHARSELECT, uisrc["CharSelect"]);
		add_sound(Sound::Name::DLGNOTICE, uisrc["DlgNotice"]);
		add_sound(Sound::Name::INVITE, uisrc["Invite"]);
		add_sound(Sound::Name::MENUDOWN, uisrc["MenuDown"]);
		add_sound(Sound::Name::MENUUP, uisrc["MenuUp"]);
		add_sound(Sound::Name::RACESELECT, uisrc["RaceSelect"]);
		add_sound(Sound::Name::SCROLLUP, uisrc["ScrollUp"]);
		add_sound(Sound::Name::SELECTMAP, uisrc["SelectMap"]);
		add_sound(Sound::Name::TAB, uisrc["Tab"]);
		add_sound(Sound::Name::WORLDSELECT, uisrc["WorldSelect"]);
		add_sound(Sound::Name::DRAGSTART, uisrc["DragStart"]);
		add_sound(Sound::Name::DRAGEND, uisrc["DragEnd"]);
		add_sound(Sound::Name::WORLDMAPOPEN, uisrc["WorldmapOpen"]);
		add_sound(Sound::Name::WORLDMAPCLOSE, uisrc["WorldmapClose"]);
		add_sound(Sound::Name::ROLLDOWN, uisrc["RollDown"]);
		add_sound(Sound::Name::ROLLUP, uisrc["RollUp"]);
		add_sound(Sound::Name::BUYSHOPITEM, uisrc["BuyShopItem"]);

		nl::node gamesrc = nl::nx::sound["Game.img"];

		add_sound(Sound::Name::GAMESTART, gamesrc["GameIn"]);
		add_sound(Sound::Name::JUMP, gamesrc["Jump"]);
		add_sound(Sound::Name::DROP, gamesrc["DropItem"]);
		add_sound(Sound::Name::PICKUP, gamesrc["PickUpItem"]);
		add_sound(Sound::Name::PORTAL, gamesrc["Portal"]);
		add_sound(Sound::Name::LEVELUP, gamesrc["LevelUp"]);
		add_sound(Sound::Name::JOBCHANGED, gamesrc["JobChanged"]);
		add_sound(Sound::Name::QUESTCLEAR, gamesrc["QuestClear"]);
		add_sound(Sound::Name::TOMBSTONE, gamesrc["Tombstone"]);
		add_sound(Sound::Name::ENCHANTSUCCESS, gamesrc["EnchantSuccess"]);
		add_sound(Sound::Name::ENCHANTFAILURE, gamesrc["EnchantFailure"]);
		add_sound(Sound::Name::MCARDGET, gamesrc["mCardGet"]);

		nl::node itemsrc = nl::nx::sound["Item.img"];

		for (auto node : itemsrc)
			add_sound(node.name(), node["Use"]);

		set_sfxvolume(configured_volume());

		return Error::Code::NONE;
	}

	void Sound::close()
	{
		for (auto &source : sources)
			source.destroy();

		sources.clear();
		//BASS_Free();
	}

	bool Sound::set_sfxvolume(uint8_t vol)
	{
		gain = std::min<uint8_t>(vol, 100) / 100.0f;

		return true;
	}

	uint8_t Sound::configured_volume()
	{
		return Setting<SFXMute>::get().load() ? 0 : Setting<SFXVolume>::get().load();
	}

	size_t Sound::add_sound(nl::node src)
	{
		nl::audio ad = src;

		auto data = reinterpret_cast<const char *>(ad.data());

		if (data)
		{
			size_t id = ad.id();

			std::string id_s = std::to_string((uint32_t) id);

			// A field effect loads its sound each time it plays.
			if (!Music::audiodb.count(id_s))
				Music::audiodb[id_s] = new membuf(data + 82, ad.length() - 82);

			return id;
		} else
		{
			return 0;
		}
	}

	void Sound::add_sound(Name name, nl::node src)
	{
		size_t id = add_sound(src);

		if (id)
			soundids[name] = id;
	}

	void Sound::add_sound(std::string itemid, nl::node src)
	{
		size_t id = add_sound(src);

		if (id)
			itemids[itemid] = id;
	}

	std::string Sound::format_id(int32_t itemid)
	{
		std::string strid = std::to_string(itemid);
		strid.insert(0, 8 - strid.size(), '0');

		return strid;
	}

	EnumMap<Sound::Name, size_t> Sound::soundids;
	std::unordered_map<std::string, size_t> Sound::itemids;

	Music::Music(std::string p)
	{
		path = p;
	}

	void Music::play(int32_t fadeout, int32_t delay, int32_t fadein, bool loop) const
	{
		static std::string bgmpath = "";

		if (path == bgmpath)
			return;

		/* will throw std::out:of:range if not used before. */
		try
		{
			audiodb.at(path);
		} catch (std::out_of_range e)
		{
			nl::audio ad = nl::nx::sound.resolve(path);

			// Later-version paths (e.g. Bgm43) aren't in this Sound.nx.
			if (!ad || ad.length() <= 82)
				return;

			auto data = reinterpret_cast<const char *>(ad.data());
			audiodb[path] = new membuf(data + 82, ad.length() - 82);
		}

		bgmpath = path;
		fadein_length = fadein;
		looping = loop;

		// v83 fades the old track out and then starts the new one (PlayBGM 0x43f301).
		if ((fadeout > 0 && music_src.isPlaying()) || delay > 0)
		{
			if (pending.empty())
			{
				fade_start = std::chrono::steady_clock::now();
				fade_length = fadeout;
			}

			start_delay = delay;
			pending = path;

			return;
		}

		pending.clear();
		start(path);
	}

	void Music::start(const std::string &path)
	{
		music_buff = Music::ctx.getBuffer(path);
		fading_in = fadein_length > 0;
		fadein_start = std::chrono::steady_clock::now();
		music_src.setGain(fading_in ? 0.0f : volume);
		music_src.setLooping(looping);
		music_src.play(music_buff);
	}

	void Music::play_once() const
	{
		static std::string bgmpath = "";

		if (path == bgmpath)
			return;

		/* will throw std::out:of:range if not used before. */
		try
		{
			audiodb.at(path);
		} catch (std::out_of_range e)
		{
			nl::audio ad = nl::nx::sound.resolve(path);

			// Later-version paths (e.g. Bgm43) aren't in this Sound.nx.
			if (!ad || ad.length() <= 82)
				return;

			auto data = reinterpret_cast<const char *>(ad.data());
			audiodb[path] = new membuf(data + 82, ad.length() - 82);
		}

		//alure::Context::MakeCurrent(ctx);
		music_buff = Music::ctx.getBuffer(path);
		music_src.setLooping(false);
		music_src.play(music_buff);

		bgmpath = path;
	}

	Error Music::init()
	{
		devMgr = alure::DeviceManager::getInstance();
		dev = devMgr.openPlayback();
		ctx = dev.createContext();
		alure::Context::MakeCurrent(ctx);
		music_src = ctx.createSource();

		alure::FileIOFactory::set(alure::MakeUnique<FileFactory>(&audiodb));
		/*TODO: add checks*/

		set_bgmvolume(configured_volume());

		return Error::Code::NONE;
	}

	bool Music::set_bgmvolume(uint8_t vol)
	{
		if (!music_src)
			return false;

		volume = std::min<uint8_t>(vol, 100) / 100.0f;
		music_src.setGain(volume);

		return true;
	}

	uint8_t Music::configured_volume()
	{
		return Setting<BGMMute>::get().load() ? 0 : Setting<BGMVolume>::get().load();
	}

	void Music::update_context()
	{
		if (!pending.empty())
		{
			auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - fade_start).count();

			if (elapsed >= std::max(fade_length, start_delay))
			{
				start(pending);
				pending.clear();
			}
			else
			{
				music_src.setGain(volume * field_effect::fade_gain(static_cast<int32_t>(elapsed), fade_length));
			}
		}
		else if (fading_in)
		{
			auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - fadein_start).count();

			if (elapsed >= fadein_length)
				fading_in = false;

			music_src.setGain(volume * field_effect::fade_in_gain(static_cast<int32_t>(elapsed), fadein_length));
		}

		if (ctx)
			ctx.update();
	}
}