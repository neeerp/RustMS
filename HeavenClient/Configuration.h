//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
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
#pragma once

#include <map>
#include <string>
#include <vector>

#include <cstdint>

#include "Template/Singleton.h"
#include "Template/TypeMap.h"
#include "Util/Misc.h"

namespace ms
{
	// Manages the 'Settings' file which contains configurations set by user behavior
	class Configuration : public Singleton<Configuration>
	{
	public:
		// Add the settings which will be used and load them
		Configuration();

		// Save.
		~Configuration();

		// Load all settings
		// If something is missing, set the default value.
		// Can be used for reloading
		void load();

		// Save the current settings.
		void save() const;

		// Get private member SHOW_FPS
		bool get_show_fps() const;

		// Get private member SHOW_PACKETS
		bool get_show_packets() const;

		// Get private member AUTO_LOGIN
		bool get_auto_login() const;

		// Get the world to login with
		uint8_t get_auto_world();

		// Get the channel to login with
		uint8_t get_auto_channel();

		// Get the account to login with
		std::string get_auto_acc();

		// Get the password to login with
		std::string get_auto_pass();

		// Get the pic to login with
		std::string get_auto_pic();

		// Get the character id to login with
		int32_t get_auto_cid();

		// Get private member TITLE
		std::string get_title() const;

		// Get private member VERSION
		std::string get_version() const;

		// Get private member JOINLINK
		std::string get_joinlink() const;

		// Get private member WEBSITE
		std::string get_website() const;

		// Get private member FINDID
		std::string get_findid() const;

		// Get private member FINDPASS
		std::string get_findpass() const;

		// Get private member RESETPIC
		std::string get_resetpic() const;

		// Get private member CHARGENX
		std::string get_chargenx() const;

		// Set private member MACS
		void set_macs(const char *macs);

		// Set private member HWID
		void set_hwid(const char *hwid, const char *volumeSerialNumber);

		// Set private member MAXWIDTH
		void set_max_width(int16_t max_width);

		// Set private member MAXHEIGHT
		void set_max_height(int16_t max_height);

		// Get private member MACS
		std::string get_macs();

		// Get private member HWID
		std::string get_hwid();

		// Get the Hard Drive Volume Serial Number
		std::string get_vol_serial_num();

		// Get the max width allowed
		int16_t get_max_width();

		// Get the max height allowed
		int16_t get_max_height();

		// Get the shop's "Right-click to sell item" boolean
		bool get_rightclicksell();

		// Set the shop's "Right-click to sell item" boolean
		void set_rightclicksell(bool value);

		// Whether to skip the startup logo sequence
		bool get_skip_startup_logos() const;

		// Whether to show the start screen
		bool get_start_shown();

		// Set whether to show the start screen
		void set_start_shown(bool value);

		// Get the character's selected world
		uint8_t get_worldid();

		// Set the character's selected world
		void set_worldid(uint8_t id);

		// Get the character's selected channel
		uint8_t get_channelid();

		// Set the character's selected channel
		void set_channelid(uint8_t id);

		// The ID the player last tried to log in with
		std::string get_login_id();

		// Keep the ID the player tries to log in with (v83 FUN_005f6952)
		void set_login_id(const std::string& id);

		// Get the known channel count for a world.
		uint8_t get_world_channel_count(uint8_t world_id);

		// Update the known channel count for a world.
		void set_world_channel_count(uint8_t world_id, uint8_t count);

		// The name the server list gave a world's channel (0-based), or
		// nothing if it gave none.
		std::string get_channel_name(uint8_t world_id, int32_t channel);

		// Update the channel names the server list gave a world.
		void set_world_channel_names(uint8_t world_id, const std::vector<std::string>& names);
		// Check if the current account is an admin account
		bool get_admin();
		// Check whether the current account is an admin account
		void set_admin(bool value);

		// Base class for an entry in the settings file
		class Entry
		{
		protected:
			Entry(const char *n, const char *v) : name(n), value(v)
			{}

			std::string name;
			std::string value;

		private:
			friend class Configuration;

			std::string to_string() const
			{
				return name + " = " + value;
			}
		};

		// Setting which converts to a boolean
		class BoolEntry : public Entry
		{
		public:
			void save(bool b);

			bool load() const;

		protected:
			using Entry::Entry;
		};

		// Setting which uses the raw string
		class StringEntry : public Entry
		{
		public:
			void save(std::string str);

			std::string load() const;

		protected:
			using Entry::Entry;
		};

		// Setting which converts to a Point<int16_t>
		class PointEntry : public Entry
		{
		public:
			void save(Point<int16_t> p);

			Point<int16_t> load() const;

		protected:
			using Entry::Entry;
		};

		// Setting which converts to an integer type.
		template<class T>
		class IntegerEntry : public Entry
		{
		public:
			void save(T num)
			{
				value = std::to_string(num);
			}

			T load() const
			{
				return string_conversion::or_zero<T>(value);
			}

		protected:
			using Entry::Entry;
		};

		// Setting which converts to a byte
		class ByteEntry : public IntegerEntry<uint8_t>
		{
		protected:
			using IntegerEntry::IntegerEntry;
		};

		// Setting which converts to a short
		class ShortEntry : public IntegerEntry<uint16_t>
		{
		protected:
			using IntegerEntry::IntegerEntry;
		};

		// Setting which converts to an int
		class IntEntry : public IntegerEntry<uint32_t>
		{
		protected:
			using IntegerEntry::IntegerEntry;
		};

		// Setting which converts to a long
		class LongEntry : public IntegerEntry<uint64_t>
		{
		protected:
			using IntegerEntry::IntegerEntry;
		};

	private:
		template <typename T>
		friend struct Setting;

		const char* FILENAME = "Settings";
		const char* TITLE = "MapleStory";
		const char* VERSION = "0.83";
		const char* JOINLINK = "https://www.nexon.com/account/en/create";
		const char* WEBSITE = "http://maplestory.nexon.net/";
		const char* FINDID = "https://www.nexon.com/account/en/login";
		const char* FINDPASS = "https://www.nexon.com/account/en/reset-password";
		const char* RESETPIC = "https://www.nexon.com/account/en/login";
		const char* CHARGENX = "https://billing.nexon.net/PurchaseNX";
		const bool SHOW_FPS = false;
		const bool SHOW_PACKETS = true;
		const bool SKIP_STARTUP_LOGOS = true;
		const bool AUTO_LOGIN = false;
		const uint8_t auto_world = 0;
		const uint8_t auto_channel = 0;
		const std::string auto_acc = "";
		const std::string auto_pass = "";
		const std::string auto_pic = "";
		const int32_t auto_cid = 0;
		bool rightclicksell = false;
		bool start_shown = false;
		std::string MACS;
		std::string HWID;
		int16_t MAXWIDTH;
		int16_t MAXHEIGHT;
		std::string VolumeSerialNumber;
		uint8_t worldid;
		uint8_t channelid;
		std::string login_id;
		std::map<uint8_t, uint8_t> world_channel_counts;
		std::map<uint8_t, std::vector<std::string>> world_channel_names;
		bool admin;
		TypeMap<Entry> settings;
	};

	// IP Address which the client will connect to
	struct ServerIP : public Configuration::StringEntry
	{
		ServerIP() : StringEntry("ServerIP", "127.0.0.1")
		{}
	};

	// Port which the client will connect to
	struct ServerPort : public Configuration::StringEntry
	{
		ServerPort() : StringEntry("ServerPort", "8484")
		{}
	};

	// Whether to start in full screen mode
	struct Fullscreen : public Configuration::BoolEntry
	{
		Fullscreen() : BoolEntry("Fullscreen", "false")
		{}
	};

	// The width of the screen
	struct Width : public Configuration::ShortEntry
	{
		Width() : ShortEntry("Width", "800")
		{}
	};

	// The height of the screen
	struct Height : public Configuration::ShortEntry
	{
		Height() : ShortEntry("Height", "600")
		{}
	};

	// Whether to use VSync
	struct VSync : public Configuration::BoolEntry
	{
		VSync() : BoolEntry("VSync", "true")
		{}
	};

	// The UI text font. The original client draws its text in Arial; Arimo has
	// the same metrics.
	struct FontPathNormal : public Configuration::StringEntry
	{
		FontPathNormal() : StringEntry("FontPathNormal", "fonts/Arimo/Arimo-Regular.ttf")
		{}
	};

	// The bold UI text font.
	struct FontPathBold : public Configuration::StringEntry
	{
		FontPathBold() : StringEntry("FontPathBold", "fonts/Arimo/Arimo-Bold.ttf")
		{}
	};

	// The chat balloon font. The original client draws balloons in 12 px Arial.
	struct FontPathBalloon : public Configuration::StringEntry
	{
		FontPathBalloon() : StringEntry("FontPathBalloon", "fonts/Arimo/Arimo-Regular.ttf")
		{}
	};

	// Music Volume
	// Number from 0 to 100
	struct BGMVolume : public Configuration::ByteEntry
	{
		BGMVolume() : ByteEntry("BGMVolume", "50")
		{}
	};

	// Sound Volume
	// Number from 0 to 100
	struct SFXVolume : public Configuration::ByteEntry
	{
		SFXVolume() : ByteEntry("SFXVolume", "50")
		{}
	};

	// Whether the music is muted, whatever BGMVolume says
	struct BGMMute : public Configuration::BoolEntry
	{
		BGMMute() : BoolEntry("BGMMute", "false")
		{}
	};

	// Whether the sound effects are muted, whatever SFXVolume says
	struct SFXMute : public Configuration::BoolEntry
	{
		SFXMute() : BoolEntry("SFXMute", "false")
		{}
	};

	// The System Option settings the client stores but doesn't apply yet.
	// Picture quality from 0 (normal) to 2 (best)
	struct PictureQuality : public Configuration::ByteEntry
	{
		PictureQuality() : ByteEntry("PictureQuality", "2")
		{}
	};

	// Mouse cursor speed from 0 (slow) to 10 (fast)
	struct MouseSpeed : public Configuration::ByteEntry
	{
		MouseSpeed() : ByteEntry("MouseSpeed", "5")
		{}
	};

	// Warn when HP falls to this percentage; 0 never warns
	struct HPAlert : public Configuration::ByteEntry
	{
		HPAlert() : ByteEntry("HPAlert", "20")
		{}
	};

	// Warn when MP falls to this percentage; 0 never warns
	struct MPAlert : public Configuration::ByteEntry
	{
		MPAlert() : ByteEntry("MPAlert", "20")
		{}
	};

	// Whether skills shake the screen
	struct ShakeScreen : public Configuration::BoolEntry
	{
		ShakeScreen() : BoolEntry("ShakeScreen", "true")
		{}
	};

	// The Game Option settings: whether to allow each kind of request. As in
	// v83 the client enforces them, turning requests down for the player.
	struct AllowWhisper : public Configuration::BoolEntry
	{
		AllowWhisper() : BoolEntry("AllowWhisper", "true")
		{}
	};

	struct AllowFriends : public Configuration::BoolEntry
	{
		AllowFriends() : BoolEntry("AllowFriends", "true")
		{}
	};

	struct AllowChatInvite : public Configuration::BoolEntry
	{
		AllowChatInvite() : BoolEntry("AllowChatInvite", "true")
		{}
	};

	struct AllowTradeRequest : public Configuration::BoolEntry
	{
		AllowTradeRequest() : BoolEntry("AllowTradeRequest", "true")
		{}
	};

	struct AllowPartyInvite : public Configuration::BoolEntry
	{
		AllowPartyInvite() : BoolEntry("AllowPartyInvite", "true")
		{}
	};

	struct AllowPartySearch : public Configuration::BoolEntry
	{
		AllowPartySearch() : BoolEntry("AllowPartySearch", "true")
		{}
	};

	// v83 has the box but never loads, saves or reads it.
	struct AllowExpeditionInvite : public Configuration::BoolEntry
	{
		AllowExpeditionInvite() : BoolEntry("AllowExpeditionInvite", "true")
		{}
	};

	struct AllowGuildChat : public Configuration::BoolEntry
	{
		AllowGuildChat() : BoolEntry("AllowGuildChat", "true")
		{}
	};

	struct AllowGuildInvite : public Configuration::BoolEntry
	{
		AllowGuildInvite() : BoolEntry("AllowGuildInvite", "true")
		{}
	};

	struct AllowAllianceChat : public Configuration::BoolEntry
	{
		AllowAllianceChat() : BoolEntry("AllowAllianceChat", "true")
		{}
	};

	struct AllowAllianceInvite : public Configuration::BoolEntry
	{
		AllowAllianceInvite() : BoolEntry("AllowAllianceInvite", "true")
		{}
	};

	struct AllowFamilyInvite : public Configuration::BoolEntry
	{
		AllowFamilyInvite() : BoolEntry("AllowFamilyInvite", "true")
		{}
	};

	// Whether to save the last used account name
	struct SaveLogin : public Configuration::BoolEntry
	{
		SaveLogin() : BoolEntry("SaveLogin", "false")
		{}
	};

	// The last used account name
	struct DefaultAccount : public Configuration::StringEntry
	{
		DefaultAccount() : StringEntry("Account", "")
		{}
	};

	// The last used world
	struct DefaultWorld : public Configuration::ByteEntry
	{
		DefaultWorld() : ByteEntry("World", "0")
		{}
	};

	// The last used channel
	struct DefaultChannel : public Configuration::ByteEntry
	{
		DefaultChannel() : ByteEntry("Channel", "0")
		{}
	};

	// The last used region
	struct DefaultRegion : public Configuration::ByteEntry
	{
		DefaultRegion() : ByteEntry("Region", "5")
		{}
	};

	// The last used character
	struct DefaultCharacter : public Configuration::ByteEntry
	{
		DefaultCharacter() : ByteEntry("Character", "0")
		{}
	};

	// Whether to show UIChatBar
	struct Chatopen : public Configuration::BoolEntry
	{
		Chatopen() : BoolEntry("Chatopen", "false")
		{}
	};

	// The height the expanded chat log was dragged to
	struct ChatHeight : public Configuration::ShortEntry
	{
		ChatHeight() : ShortEntry("ChatHeight", "70")
		{}
	};

	// The default position of UIStatsInfo
	struct PosSTATS : public Configuration::PointEntry
	{
		PosSTATS() : PointEntry("PosSTATS", "(72,72)")
		{}
	};

	// The default position of UIEquipInventory
	struct PosEQINV : public Configuration::PointEntry
	{
		PosEQINV() : PointEntry("PosEQINV", "(250,160)")
		{}
	};

	// The default position of UIItemInventory
	struct PosINV : public Configuration::PointEntry
	{
		PosINV() : PointEntry("PosINV", "(300,160)")
		{}
	};

	// The default position of UISkillBook
	struct PosSKILL : public Configuration::PointEntry
	{
		PosSKILL() : PointEntry("PosSKILL", "(96,96)")
		{}
	};

	// The default position of UIMonsterBook
	struct PosMONSTERBOOK : public Configuration::PointEntry
	{
		PosMONSTERBOOK() : PointEntry("PosMONSTERBOOK", "(162,90)")
		{}
	};

	// The default position of UIQuestLog
	struct PosQUEST : public Configuration::PointEntry
	{
		PosQUEST() : PointEntry("PosQUEST", "(300,160)")
		{}
	};

	// The default position of UIWorldMap
	struct PosMAP : public Configuration::PointEntry
	{
		PosMAP() : PointEntry("PosMAP", "(66,0)")
		{}
	};

	// The default position of UIUserList
	struct PosUSERLIST : public Configuration::PointEntry
	{
		PosUSERLIST() : PointEntry("PosUSERLIST", "(104, 104)")
		{}
	};

	// The default position of UIChatBar
	struct PosCHAT : public Configuration::PointEntry
	{
		PosCHAT() : PointEntry("PosCHAT", "(0, 572)")
		{}
	};

	// The default position of UIMiniMap
	struct PosMINIMAP : public Configuration::PointEntry
	{
		PosMINIMAP() : PointEntry("PosMINIMAP", "(0, 0)")
		{}
	};

	// The default position of UIShop
	struct PosSHOP : public Configuration::PointEntry
	{
		PosSHOP() : PointEntry("PosSHOP", "(146, 48)")
		{}
	};

	// The default position of UIStorage: centred, like the v83 dialog
	struct PosSTORAGE : public Configuration::PointEntry
	{
		PosSTORAGE() : PointEntry("PosSTORAGE", "(168, 141)")
		{}
	};

	// The default position of UITradingRoom: centred, like the v83 dialog
	struct PosTRADE : public Configuration::PointEntry
	{
		PosTRADE() : PointEntry("PosTRADE", "(117, 63)")
		{}
	};

	// The default position of UIPersonalShop: centred, like the v83 dialog
	struct PosPERSONALSHOP : public Configuration::PointEntry
	{
		PosPERSONALSHOP() : PointEntry("PosPERSONALSHOP", "(137, 23)")
		{}
	};

	// The default position of UINotice
	struct PosNOTICE : public Configuration::PointEntry
	{
		PosNOTICE() : PointEntry("PosNOTICE", "(400, 285)")
		{}
	};

	// The default position of UIMessenger
	struct PosMESSENGER : public Configuration::PointEntry
	{
		PosMESSENGER() : PointEntry("PosMESSENGER", "(250, 80)")
		{}
	};

	// The default position of UIChannel
	struct PosCHANNEL : public Configuration::PointEntry
	{
		PosCHANNEL() : PointEntry("PosCHANNEL", "(215, 100)")
		{}
	};

	// The default position of UIKeyConfig
	struct PosKEYCONFIG : public Configuration::PointEntry
	{
		PosKEYCONFIG() : PointEntry("PosKEYCONFIG", "(65, 50)")
		{}
	};

	// The default position of UIGameOption
	struct PosGAMEOPTION : public Configuration::PointEntry
	{
		PosGAMEOPTION() : PointEntry("PosGAMEOPTION", "(321, 158)")
		{}
	};

	// The default position of UISystemOption
	struct PosSYSTEMOPTION : public Configuration::PointEntry
	{
		PosSYSTEMOPTION() : PointEntry("PosSYSTEMOPTION", "(250, 117)")
		{}
	};

	// The default position of UICharInfo
	struct PosCHARINFO : public Configuration::PointEntry
	{
		PosCHARINFO() : PointEntry("PosCHARINFO", "(264, 264)") {}
	};

	// Where UIQuestHelper was moved to. Until then, as in the original, it
	// is unset and the helper takes its default place.
	struct PosQUESTHELPER : public Configuration::PointEntry
	{
		PosQUESTHELPER() : PointEntry("PosQUESTHELPER", "(-1,-1)") {}
	};

	// Where v83 first puts the Energy Charge gauge isn't known; this is above the status bar's middle.
	struct PosENERGYBAR : public Configuration::PointEntry
	{
		PosENERGYBAR() : PointEntry("PosENERGYBAR", "(359,500)") {}
	};

	// The type of UIMiniMap on maps that aren't towns: 0 folded, 1 the map
	// alone, 2 the map under its names
	struct MiniMapType : public Configuration::ByteEntry
	{
		MiniMapType() : ByteEntry("MiniMapType", "2")
		{}
	};

	// Whether to use a simple version of UIMiniMap
	struct MiniMapSimpleMode : public Configuration::BoolEntry
	{
		MiniMapSimpleMode() : BoolEntry("MiniMapSimpleMode", "false")
		{}
	};

	// Whether to use default helpers for UIMiniMap
	struct MiniMapDefaultHelpers : public Configuration::BoolEntry
	{
		MiniMapDefaultHelpers() : BoolEntry("MiniMapDefaultHelpers", "false")
		{}
	};

	template<typename T>
	// Can be used to access settings.
	struct Setting
	{
		// Access a setting.
		static T &get()
		{
			static_assert(std::is_base_of<Configuration::Entry, T>::value,
						  "template parameter T for Setting must inherit from Configuration::Entry.");

			auto *entry = Configuration::get().settings.get<T>();

			if (entry)
			{
				return *entry;
			} else
			{
				static T defaultentry;
				return defaultentry;
			}
		}
	};
}
